//! The driver: the only code in `tests/` that knows how the app is built
//! and run. A test launches a [`Reader`], does what a person at the window
//! would do, and asks what that person could see.
//!
//! The app is reached through one function, `md_viewer::app`, and run
//! without a window by `egui_kittest`. Nothing here names a type, module or
//! function inside the app, so the tests keep compiling when those change.

mod act;
mod figures;
mod folder;
mod keys;
mod paint;
mod pointer;
mod see;
mod web;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;

use self::folder::Folder;
use self::paint::Pixels;
pub(crate) use self::paint::Rgb;
pub(crate) use self::see::Area;
pub(crate) use self::web::serve;

/// The window every launch starts with, in pixels, unless the test asks
/// for another.
const WINDOW: (f32, f32) = (1200.0, 800.0);

/// The largest picture, in pixels a side, the screen can be handed: what
/// an ordinary graphics card takes.
const MAX_PICTURE_SIDE: usize = 16_384;

/// The frames an action gets to come to rest before the driver starts
/// waiting on the clock for the app's background work.
const QUICK_FRAMES: u32 = 12;

/// How long the driver waits for background work: `WAIT_FRAMES` frames,
/// `WAIT_STEP` apart, is twenty seconds.
const WAIT_FRAMES: u32 = 2000;
const WAIT_STEP: Duration = Duration::from_millis(10);

/// The name md-viewer keeps its session under in the settings file.
const SESSION_KEY: &str = "md-viewer-state";

/// What eframe keeps between runs, held in memory instead of in the
/// user's data directory.
#[derive(Default)]
struct Session(HashMap<String, String>);

impl eframe::Storage for Session {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    fn flush(&mut self) {}
}

/// The app as eframe sees it, so `egui_kittest` can run it.
struct Running {
    app: Box<dyn eframe::App>,
    /// Off while the harness is being built. The harness runs frames of
    /// its own then and throws their output away; the app's first frame,
    /// which titles the window, has to be one the driver sees.
    shown: bool,
}

impl eframe::App for Running {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.shown {
            self.app.update(ctx, frame);
        }
    }
}

/// A launch being prepared: the files on disk, the command line, and the
/// window size. [`Launch::open`] starts the app.
pub(crate) struct Launch {
    folder: Folder,
    session: Session,
    args: Vec<String>,
    window: (f32, f32),
}

impl Launch {
    /// Put the text document `name` in the folder before the app starts.
    /// A name may hold subfolders.
    pub(crate) fn doc(self, name: &str, content: &str) -> Self {
        self.folder.write(name, content);
        self
    }

    /// Put the file `name` with these exact bytes in the folder.
    pub(crate) fn bytes(self, name: &str, content: &[u8]) -> Self {
        self.folder.write(name, content);
        self
    }

    /// The command line after `md-viewer`, typed in the folder: an argument
    /// that is not a flag is a path relative to it.
    pub(crate) fn args(mut self, args: &[&str]) -> Self {
        self.args = args
            .iter()
            .map(|arg| {
                if arg.starts_with('-') {
                    (*arg).to_owned()
                } else {
                    self.folder.path(arg).to_string_lossy().into_owned()
                }
            })
            .collect();
        self
    }

    /// The command line after `md-viewer`, passed on untouched, so a
    /// relative path means what it means to the test process.
    pub(crate) fn raw_args(mut self, args: &[&str]) -> Self {
        self.args = args.iter().map(|arg| (*arg).to_owned()).collect();
        self
    }

    /// Where `name` will be on disk, for a session that has to name it.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.folder.path(name)
    }

    /// Date the file `name` as last changed `seconds` ago.
    #[track_caller]
    pub(crate) fn aged(self, name: &str, seconds: u64) -> Self {
        self.folder.age(name, Duration::from_secs(seconds));
        self
    }

    /// Make `name` an empty folder before the app starts.
    pub(crate) fn folder(self, name: &str) -> Self {
        self.folder.make_folder(name);
        self
    }

    /// Make `name` a symbolic link to `target`, another name in the folder.
    #[cfg(unix)]
    pub(crate) fn link(self, name: &str, target: &str) -> Self {
        self.folder.link(name, target);
        self
    }

    /// Start as if the last run had left `saved` as its session: the text
    /// md-viewer keeps in its settings file between runs.
    pub(crate) fn remembering(mut self, saved: &str) -> Self {
        let saved = saved.to_owned();
        self.session.0.insert(SESSION_KEY.to_owned(), saved);
        self
    }

    /// The window's size in pixels.
    pub(crate) const fn window(mut self, width: f32, height: f32) -> Self {
        self.window = (width, height);
        self
    }

    /// Start the app and let it come to rest.
    #[track_caller]
    pub(crate) fn open(self) -> Reader {
        match self.try_open() {
            Ok(reader) => reader,
            Err(refusal) => panic!("md-viewer refused its command line: {refusal}"),
        }
    }

    /// Start the app, or return what it printed when it refused the
    /// command line.
    pub(crate) fn try_open(self) -> Result<Reader, String> {
        let Self {
            folder,
            session,
            args,
            window,
        } = self;
        let mut refusal = None;
        let kit = egui_kittest::Harness::builder()
            .with_size(egui::vec2(window.0, window.1))
            .build_eframe(|kit_creation| {
                // A context of this launch's own, so the session can be
                // lent to the app for exactly as long as it is built.
                let mut creation =
                    eframe::CreationContext::_new_kittest(kit_creation.egui_ctx.clone());
                creation.storage = Some(&session);
                let app = md_viewer::app(&args, &creation).unwrap_or_else(|error| {
                    refusal = Some(error.to_string());
                    Box::new(Refused)
                });
                Running { app, shown: false }
            });
        if let Some(refusal) = refusal {
            return Err(refusal);
        }
        let mut reader = Reader {
            kit,
            folder,
            session,
            window,
            title: None,
            quit_asked: false,
            pixels: Pixels::default(),
            opened: Vec::new(),
            copied: None,
            held: None,
        };
        reader.kit.state_mut().shown = true;
        reader.settle();
        Ok(reader)
    }
}

/// Stands in for the app in a harness that is thrown away because the
/// command line was refused.
struct Refused;

impl eframe::App for Refused {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}

/// The app running on a folder of documents, with a person in front of it.
pub(crate) struct Reader {
    kit: egui_kittest::Harness<'static, Running>,
    folder: Folder,
    session: Session,
    window: (f32, f32),
    /// The last title the app gave its window.
    title: Option<String>,
    quit_asked: bool,
    /// The pictures the app has handed to the screen.
    pixels: Pixels,
    /// The addresses the app has asked the desktop to open, oldest first.
    opened: Vec<String>,
    /// What the app last put on the clipboard.
    copied: Option<String>,
    /// Where the pointer is while a mouse button is held down, in points.
    held: Option<egui::Pos2>,
}

impl Reader {
    /// An empty folder and no command line yet.
    pub(crate) fn launch() -> Launch {
        Launch {
            folder: Folder::new(),
            session: Session::default(),
            args: Vec::new(),
            window: WINDOW,
        }
    }

    /// Write each `(name, content)` document and run `md-viewer <first>`.
    pub(crate) fn with_docs(docs: &[(&str, &str)]) -> Self {
        let mut launch = Self::launch();
        for (name, content) in docs {
            launch = launch.doc(name, content);
        }
        let first: Vec<&str> = docs.iter().take(1).map(|(name, _)| *name).collect();
        launch.args(&first).open()
    }

    /// Run `md-viewer` with no arguments on a machine that has never run
    /// it.
    pub(crate) fn first_run() -> Self {
        Self::launch().open()
    }

    /// Close the window the way quitting does, which saves the session,
    /// then run `md-viewer <args>` again in the same folder.
    pub(crate) fn restart(self, args: &[&str]) -> Self {
        let Self {
            mut kit,
            folder,
            mut session,
            window,
            ..
        } = self;
        kit.state_mut().app.save(&mut session);
        drop(kit);
        let launch = Launch {
            folder,
            session,
            args: Vec::new(),
            window,
        };
        launch.args(args).open()
    }

    /// Where `name` is on disk.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.folder.path(name)
    }

    /// The folder the documents are in.
    pub(crate) fn folder(&self) -> &Path {
        self.folder.root()
    }

    /// Write `content` to `name` behind the app's back, as another program
    /// would.
    pub(crate) fn write_file(&self, name: &str, content: &str) {
        self.folder.write(name, content);
    }

    /// Write these exact bytes to `name` behind the app's back.
    pub(crate) fn write_bytes(&self, name: &str, content: &[u8]) {
        self.folder.write(name, content);
    }

    /// Replace `name` the way most editors save: write the new text beside
    /// it, then rename that over it.
    #[track_caller]
    pub(crate) fn save_over(&self, name: &str, content: &str) {
        let beside = format!("{name}.editor-swap");
        self.folder.write(&beside, content);
        self.rename_file(&beside, name);
    }

    /// Give the file or folder `from` the name `to` behind the app's back.
    #[track_caller]
    pub(crate) fn rename_file(&self, from: &str, to: &str) {
        std::fs::rename(self.folder.path(from), self.folder.path(to)).unwrap();
    }

    /// Make the folder `name` behind the app's back.
    pub(crate) fn make_folder(&self, name: &str) {
        self.folder.make_folder(name);
    }

    /// What `name` holds on disk now.
    #[track_caller]
    pub(crate) fn read_file(&self, name: &str) -> String {
        std::fs::read_to_string(self.folder.path(name)).unwrap()
    }

    /// The exact bytes `name` holds on disk now.
    #[track_caller]
    pub(crate) fn read_bytes(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.folder.path(name)).unwrap()
    }

    /// The names of everything in the folder `name` (`""` for the top),
    /// sorted.
    #[track_caller]
    pub(crate) fn files_in(&self, name: &str) -> Vec<String> {
        let entries = std::fs::read_dir(self.folder.path(name)).unwrap();
        let mut names: Vec<String> = entries
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Delete `name` behind the app's back.
    #[track_caller]
    pub(crate) fn delete_file(&self, name: &str) {
        std::fs::remove_file(self.folder.path(name)).unwrap();
    }

    /// Delete the folder `name` and all in it behind the app's back.
    #[track_caller]
    pub(crate) fn delete_folder(&self, name: &str) {
        std::fs::remove_dir_all(self.folder.path(name)).unwrap();
    }

    /// How many pixels a point of the app's layout takes: the zoom.
    fn scale(&self) -> f32 {
        self.kit.ctx.pixels_per_point()
    }

    /// Where a pixel of the window is in the app's layout.
    fn to_points(&self, pixel: egui::Pos2) -> egui::Pos2 {
        (pixel.to_vec2() / self.scale()).to_pos2()
    }

    /// Run one frame on `events`, through the input hook and the update
    /// call eframe makes, and note what the frame asked of the window.
    fn frame(&mut self, modifiers: egui::Modifiers, events: Vec<egui::Event>) {
        let mut input = self.kit.input_mut().take();
        // The window keeps its size in pixels, so zooming in leaves the app
        // fewer points to lay out in, as it does on a real screen.
        let size = egui::vec2(self.window.0, self.window.1) / self.scale();
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size));
        input.max_texture_side = Some(MAX_PICTURE_SIDE);
        input.modifiers = modifiers;
        input.events = events;
        let ctx = self.kit.ctx.clone();
        self.kit.state_mut().app.raw_input_hook(&ctx, &mut input);
        *self.kit.input_mut() = input;
        self.kit.step();
        self.note_window_requests();
    }

    /// Remember what the last frame asked of the window and the desktop.
    fn note_window_requests(&mut self) {
        let output = self.kit.output();
        self.pixels.note(&output.textures_delta);
        for command in &output.platform_output.commands {
            match command {
                egui::OutputCommand::OpenUrl(open) => self.opened.push(open.url.clone()),
                egui::OutputCommand::CopyText(text) => self.copied = Some(text.clone()),
                egui::OutputCommand::CopyImage(_) => {}
            }
        }
        for viewport in output.viewport_output.values() {
            for command in &viewport.commands {
                match command {
                    egui::ViewportCommand::Title(title) => self.title = Some(title.clone()),
                    egui::ViewportCommand::Close => self.quit_asked = true,
                    _ => {}
                }
            }
        }
    }

    /// Run frames until the app stops asking for an immediate repaint.
    /// Past the first few frames each one waits, because what the app is
    /// still waiting for by then is a background thread.
    #[track_caller]
    fn settle(&mut self) {
        let settled = (0..WAIT_FRAMES).any(|frame| {
            if frame >= QUICK_FRAMES {
                std::thread::sleep(WAIT_STEP);
            }
            self.frame(egui::Modifiers::NONE, Vec::new());
            let root = &self.kit.output().viewport_output[&egui::ViewportId::ROOT];
            !root.repaint_delay.is_zero()
        });
        assert!(
            settled,
            "the app never came to rest: {:?}",
            self.kit.ctx.repaint_causes()
        );
    }

    /// Keep the app running until `seen` holds, for what arrives from a
    /// background thread or the file system. Fails after twenty seconds.
    #[track_caller]
    pub(crate) fn wait_for(&mut self, what: &str, seen: impl Fn(&Self) -> bool) {
        let arrived = (0..WAIT_FRAMES).any(|_| {
            if seen(self) {
                return true;
            }
            std::thread::sleep(WAIT_STEP);
            self.frame(egui::Modifiers::NONE, Vec::new());
            false
        });
        assert!(arrived, "timed out waiting for {what}\n{}", self.describe());
        self.settle();
    }

    /// Keep the app running for `time`, for asserting that something does
    /// not happen.
    pub(crate) fn idle_for(&mut self, time: Duration) {
        let frames = time.as_millis() / WAIT_STEP.as_millis();
        for _ in 0..frames {
            std::thread::sleep(WAIT_STEP);
            self.frame(egui::Modifiers::NONE, Vec::new());
        }
    }
}
