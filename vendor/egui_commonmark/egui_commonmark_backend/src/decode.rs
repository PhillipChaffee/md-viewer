//! Decodes GIF and WebP pictures away from the thread that draws.
//!
//! egui's own loaders for the two decode a whole picture inside the frame
//! that first asks for it: every frame of an animation, on the thread that
//! draws the window. [`Decoder`] is asked before them. It hands the file to
//! one thread that decodes one picture at a time, answers "pending" until
//! that thread is done, and has the thread ask for one repaint then.
//!
//! A picture that is forgotten while it waits or is being decoded is not
//! kept when the thread gets to the end of it: see [`Decoder::forget`].

use std::{
    collections::{HashMap, VecDeque},
    io::Cursor,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use egui::{
    decode_animated_image_uri, has_gif_magic_header, has_webp_header,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage, FrameDurations, Id,
};
use image::{
    codecs::{gif::GifDecoder, webp::WebPDecoder},
    AnimationDecoder as _,
};

/// A picture that was decoded: its frames, and how long each is shown.
struct Decoded {
    frames: Vec<Arc<ColorImage>>,
    delays: FrameDurations,
}

impl Decoded {
    fn bytes(&self) -> usize {
        let pixels = self.frames.iter().map(|frame| frame.pixels.len());
        pixels.sum::<usize>() * size_of::<egui::Color32>()
    }
}

/// Decode the GIF or WebP in `file`. A picture whose frames take more than
/// `most` bytes is cut down to its first frame, as soon as that is known:
/// it would not be played. Decoding stops with an error once `wanted` says
/// that nobody waits for the picture.
fn decode(file: &[u8], most: u64, wanted: &dyn Fn() -> bool) -> Result<Decoded, String> {
    let frames = if has_gif_magic_header(file) {
        let decoder = GifDecoder::new(Cursor::new(file)).map_err(|err| err.to_string())?;
        decoder.into_frames()
    } else {
        let mut decoder = WebPDecoder::new(Cursor::new(file)).map_err(|err| err.to_string())?;
        if !decoder.has_animation() {
            return still(file);
        }
        // What egui's loader draws an animation on: nothing.
        decoder
            .set_background_color(image::Rgba([0, 0, 0, 0]))
            .map_err(|err| err.to_string())?;
        decoder.into_frames()
    };
    let (mut kept, mut delays, mut bytes) = (Vec::new(), Vec::new(), 0_u64);
    for frame in frames {
        if !wanted() {
            return Err("the picture was forgotten".to_owned());
        }
        let frame = frame.map_err(|err| err.to_string())?;
        let pixels = frame.buffer();
        let size = [pixels.width() as usize, pixels.height() as usize];
        bytes += (size[0] * size[1] * size_of::<egui::Color32>()) as u64;
        if bytes > most && !kept.is_empty() {
            kept.truncate(1);
            delays.truncate(1);
            break;
        }
        kept.push(Arc::new(ColorImage::from_rgba_unmultiplied(
            size,
            pixels.as_flat_samples().as_slice(),
        )));
        delays.push(Duration::from(frame.delay()));
    }
    if kept.is_empty() {
        return Err("the picture has no frames".to_owned());
    }
    Ok(Decoded {
        frames: kept,
        delays: FrameDurations::new(delays),
    })
}

/// A WebP that does not move, as a picture of one frame.
fn still(file: &[u8]) -> Result<Decoded, String> {
    let picture = image::load_from_memory_with_format(file, image::ImageFormat::WebP)
        .map_err(|err| err.to_string())?
        .to_rgba8();
    let size = [picture.width() as usize, picture.height() as usize];
    let frame = ColorImage::from_rgba_unmultiplied(size, picture.as_flat_samples().as_slice());
    Ok(Decoded {
        frames: vec![Arc::new(frame)],
        delays: FrameDurations::new(vec![Duration::ZERO]),
    })
}

/// What is known of one picture, by its address.
#[derive(Clone)]
enum Entry {
    /// It waits for the thread or is being decoded. The number tells this
    /// request from an earlier one for the same address.
    Coming(u64),
    Ready(Arc<Decoded>),
    Failed(String),
}

type Entries = Arc<Mutex<HashMap<String, Entry>>>;

/// One picture to decode.
struct Job {
    uri: String,
    file: Bytes,
    request: u64,
    /// See [`decode`].
    most: u64,
    /// Asked for a repaint once the picture is decoded. The job carries it,
    /// so that the thread holds on to no window while it has nothing to do.
    ctx: egui::Context,
}

impl Job {
    fn is_wanted(&self, entries: &Entries) -> bool {
        matches!(entries.lock().get(&self.uri), Some(Entry::Coming(request)) if *request == self.request)
    }

    /// Decode the picture and keep the result, unless the picture was
    /// forgotten or asked for again in the meantime. Returns whether the
    /// result was kept.
    fn run(&self, entries: &Entries) -> bool {
        let wanted = || self.is_wanted(entries);
        if !wanted() {
            return false;
        }
        // A decoder that panics on a broken file must not take the thread,
        // and every later picture, with it.
        let decoded = catch_unwind(AssertUnwindSafe(|| decode(&self.file, self.most, &wanted)))
            .unwrap_or_else(|_| Err("the decoder gave up on the picture".to_owned()));
        let decoded = match decoded {
            Ok(decoded) => Entry::Ready(Arc::new(decoded)),
            Err(err) => Entry::Failed(err),
        };
        // Looked at and written under one lock: a picture forgotten since
        // the last look must not come back.
        let mut entries = entries.lock();
        let wanted = matches!(entries.get(&self.uri), Some(Entry::Coming(request)) if *request == self.request);
        if wanted {
            entries.insert(self.uri.clone(), decoded);
        }
        wanted
    }
}

/// The thread that decodes, started when the first picture is asked for.
/// It ends when the [`Decoder`] is dropped.
fn start_thread(entries: Entries) -> Option<mpsc::Sender<Job>> {
    let (jobs, queue) = mpsc::channel::<Job>();
    let decode_all = move || {
        for job in queue {
            // The lock on the entries is let go before the window is
            // asked for anything: the window asks the entries whether a
            // picture is pending while it holds its own lock.
            if job.run(&entries) {
                job.ctx.request_repaint();
            }
        }
    };
    let thread = std::thread::Builder::new().name("egui_commonmark::decode".to_owned());
    thread.spawn(decode_all).ok().map(|_| jobs)
}

/// Where the jobs go.
#[derive(Default)]
struct Queue {
    thread: Option<mpsc::Sender<Job>>,
    /// The jobs of a window whose pictures are decoded by hand: see
    /// [`decode_by_hand`].
    held: VecDeque<Job>,
}

/// The image loader that decodes GIF and WebP pictures on a thread of its
/// own.
#[derive(Default)]
pub(crate) struct Decoder {
    entries: Entries,
    requests: AtomicU64,
    queue: Mutex<Queue>,
}

fn by_hand_id() -> Id {
    Id::new("egui_commonmark pictures are decoded by hand")
}

fn decoder_id() -> Id {
    Id::new("egui_commonmark picture decoder")
}

impl Decoder {
    const ID: &'static str = egui::generate_loader_id!(Decoder);

    fn send(&self, job: Job) {
        let mut queue = self.queue.lock();
        if job.ctx.data(|data| data.get_temp(by_hand_id())) == Some(true) {
            queue.held.push_back(job);
            return;
        }
        if queue.thread.is_none() {
            queue.thread = start_thread(Arc::clone(&self.entries));
        }
        let sent = queue.thread.as_ref().map(|thread| thread.send(job));
        if !matches!(sent, Some(Ok(()))) {
            // No thread could be started. The picture is left pending,
            // which holds its place, and is asked for again once it has
            // been forgotten.
            queue.thread = None;
        }
    }
}

impl ImageLoader for Decoder {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, frame_uri: &str, _: SizeHint) -> ImageLoadResult {
        let (uri, index) =
            decode_animated_image_uri(frame_uri).map_err(|_| LoadError::NotSupported)?;
        // The lock is let go before the window is asked for anything.
        let known = self.entries.lock().get(uri).cloned();
        match known {
            Some(Entry::Coming(_)) => return Ok(ImagePoll::Pending { size: None }),
            Some(Entry::Failed(err)) => return Err(LoadError::Loading(err)),
            Some(Entry::Ready(decoded)) => {
                let image = Arc::clone(&decoded.frames[index % decoded.frames.len()]);
                // Where egui's own loaders leave the delays, and where the
                // renderer looks for them.
                let delays = decoded.delays.clone();
                ctx.data_mut(|data| data.insert_temp(Id::new(uri), delays));
                return Ok(ImagePoll::Ready { image });
            }
            None => {}
        }
        let file = match ctx.try_load_bytes(uri)? {
            BytesPoll::Ready { bytes, .. } => bytes,
            BytesPoll::Pending { size } => return Ok(ImagePoll::Pending { size }),
        };
        if !has_gif_magic_header(&file) && !has_webp_header(&file) {
            return Err(LoadError::NotSupported);
        }
        let request = self.requests.fetch_add(1, Ordering::Relaxed);
        let coming = Entry::Coming(request);
        self.entries.lock().insert(uri.to_owned(), coming);
        self.send(Job {
            uri: uri.to_owned(),
            file,
            request,
            most: crate::animation::most_for_one_picture(ctx),
            ctx: ctx.clone(),
        });
        Ok(ImagePoll::Pending { size: None })
    }

    /// Forget the picture at `uri`. If it is being decoded, or waits to be,
    /// the result is dropped when the thread gets to it: the thread stops
    /// at the next frame of a picture it is in the middle of.
    fn forget(&self, uri: &str) {
        self.entries.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.entries.lock().clear();
    }

    fn byte_size(&self) -> usize {
        let entries = self.entries.lock();
        let sizes = entries.values().map(|entry| match entry {
            Entry::Ready(decoded) => decoded.bytes(),
            Entry::Failed(err) => err.len(),
            Entry::Coming(_) => 0,
        });
        sizes.sum()
    }

    fn has_pending(&self) -> bool {
        let entries = self.entries.lock();
        entries
            .values()
            .any(|entry| matches!(entry, Entry::Coming(_)))
    }
}

/// Have `ctx` ask [`Decoder`] for GIF and WebP pictures before it asks
/// egui's own loaders, which must be installed already: egui asks the
/// loader added last first.
pub(crate) fn install(ctx: &egui::Context) {
    if ctx.is_loader_installed(Decoder::ID) {
        return;
    }
    let decoder = Arc::new(Decoder::default());
    ctx.add_image_loader(Arc::clone(&decoder) as Arc<dyn ImageLoader + Send + Sync>);
    ctx.data_mut(|data| data.insert_temp(decoder_id(), decoder));
}

/// Have the pictures of `ctx` decoded by [`decode_held`] and by no thread,
/// from now on. For tests: a thread is done whenever it is done, and no
/// count of frames tells a test that it is.
pub fn decode_by_hand(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(by_hand_id(), true));
}

/// Decode the pictures `ctx` has asked for since the last call, as the
/// thread would have, and return how many were kept. See
/// [`decode_by_hand`].
pub fn decode_held(ctx: &egui::Context) -> usize {
    let Some(decoder) = ctx.data(|data| data.get_temp::<Arc<Decoder>>(decoder_id())) else {
        return 0;
    };
    let held = std::mem::take(&mut decoder.queue.lock().held);
    held.iter().filter(|job| job.run(&decoder.entries)).count()
}
