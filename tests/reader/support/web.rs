//! A web server on this machine, for documents that load pictures over
//! HTTP. It listens on a port of its own and never leaves the loopback
//! interface.

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// A web server on this machine, and what it has been asked for.
pub(crate) struct Site {
    /// Where its files are, such as `http://127.0.0.1:40123`.
    pub(crate) address: String,
    asked_for: Arc<Mutex<Vec<String>>>,
}

impl Site {
    /// How many times the file named `name` has been asked for.
    pub(crate) fn times_asked_for(&self, name: &str) -> usize {
        let asked_for = self.asked_for.lock().unwrap();
        asked_for.iter().filter(|path| path.ends_with(name)).count()
    }
}

/// Answer one request on `stream`: the bytes of the file whose name the
/// path ends with, or a 404. The path is noted in `asked_for`.
fn answer(
    stream: TcpStream,
    files: &[(String, Vec<u8>)],
    asked_for: &Mutex<Vec<String>>,
) -> std::io::Result<()> {
    let mut request = String::new();
    BufReader::new(&stream).read_line(&mut request)?;
    let path = request.split_whitespace().nth(1).unwrap_or_default();
    asked_for.lock().unwrap().push(path.to_owned());
    let found = files.iter().find(|(name, _)| path.ends_with(name.as_str()));
    let mut stream = stream;
    match found {
        Some((_, bytes)) => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )?;
            stream.write_all(bytes)
        }
        None => stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
    }
}

/// Serve `files` over HTTP until the tests end.
pub(crate) fn site(files: &[(&str, &[u8])]) -> Site {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let files: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect();
    let asked_for = Arc::new(Mutex::new(Vec::new()));
    let noted = Arc::clone(&asked_for);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            // A reader that hangs up early is no concern of the server.
            let _ = answer(stream, &files, &noted);
        }
    });
    Site { address, asked_for }
}

/// Serve `files` over HTTP until the tests end, and return the address
/// they are under, such as `http://127.0.0.1:40123`.
pub(crate) fn serve(files: &[(&str, &[u8])]) -> String {
    site(files).address
}
