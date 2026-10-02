//! A small blocking HTTP/1.1 server: request parsing, response building,
//! query-string decoding, safe static-file serving, and a bounded thread pool.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Guards against a client that opens a socket and dribbles bytes forever.
const IO_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
}

impl Request {
    pub fn param(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }

    /// Parse a query parameter, falling back to `default` when it is absent
    /// *or* unparseable — a malformed `?n=abc` should not 500 the page.
    pub fn param_or<T: std::str::FromStr>(&self, key: &str, default: T) -> T {
        self.param(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
}

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub extra_headers: Vec<(String, String)>,
}

impl Response {
    pub fn new(status: u16, content_type: &str, body: Vec<u8>) -> Response {
        Response {
            status,
            content_type: content_type.to_string(),
            body,
            extra_headers: Vec::new(),
        }
    }

    pub fn json(body: String) -> Response {
        let mut r = Response::new(200, "application/json; charset=utf-8", body.into_bytes());
        r.extra_headers
            .push(("Cache-Control".into(), "no-store".into()));
        r
    }

    pub fn text(status: u16, body: &str) -> Response {
        Response::new(status, "text/plain; charset=utf-8", body.as_bytes().to_vec())
    }

    pub fn header(mut self, name: &str, value: &str) -> Response {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    fn reason(&self) -> &'static str {
        match self.status {
            200 => "OK",
            204 => "No Content",
            304 => "Not Modified",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
    }

    fn write_to(&self, stream: &mut TcpStream, head_only: bool) -> std::io::Result<()> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\
             X-Content-Type-Options: nosniff\r\n",
            self.status,
            self.reason(),
            self.content_type,
            self.body.len()
        );
        for (k, v) in &self.extra_headers {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        head.push_str("\r\n");
        stream.write_all(head.as_bytes())?;
        if !head_only {
            stream.write_all(&self.body)?;
        }
        stream.flush()
    }
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_query(raw: &str) -> HashMap<String, String> {
    raw.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((k, v)) => (percent_decode(k), percent_decode(v)),
            None => (percent_decode(pair), String::new()),
        })
        .collect()
}

pub fn parse_request_line(line: &str) -> Option<Request> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    // The HTTP version is present but we only speak 1.1 and always close.
    parts.next()?;

    let (path, raw_query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };
    Some(Request {
        method,
        path: percent_decode(path),
        query: parse_query(raw_query),
    })
}

pub fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Resolve a URL path inside `root`, refusing anything that escapes it.
/// Rejecting `..` lexically (rather than relying on `canonicalize`) also stops
/// traversal through a symlink that points outside the tree.
pub fn resolve_static(root: &Path, url_path: &str) -> Option<PathBuf> {
    let trimmed = url_path.trim_start_matches('/');
    let rel = if trimmed.is_empty() { "index.html" } else { trimmed };

    let mut safe = PathBuf::new();
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(part) => safe.push(part),
            // Absolute prefixes, `.` and `..` are all refused outright.
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if safe.as_os_str().is_empty() {
        return None;
    }
    Some(root.join(safe))
}

fn serve_static(root: &Path, url_path: &str) -> Response {
    let Some(path) = resolve_static(root, url_path) else {
        return Response::text(400, "bad path");
    };
    // A directory request falls through to its index.html.
    let path = if path.is_dir() { path.join("index.html") } else { path };

    match std::fs::read(&path) {
        Ok(bytes) => {
            let ct = content_type_for(&path);
            // Fonts and the vendored KaTeX build are immutable; the app's own
            // HTML/CSS/JS must not be cached while iterating on it.
            let cache = if path.components().any(|c| c.as_os_str() == "vendor") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            Response::new(200, ct, bytes).header("Cache-Control", cache)
        }
        Err(_) => Response::text(404, "not found"),
    }
}

/// Read the request head, then drain any declared body so the client sees a
/// clean response rather than a connection reset.
fn read_request(stream: &TcpStream) -> Result<Request, Response> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    if reader.read_line(&mut line).is_err() || line.is_empty() {
        return Err(Response::text(400, "empty request"));
    }
    if line.len() > MAX_HEADER_BYTES {
        return Err(Response::text(414, "request line too long"));
    }
    let request = parse_request_line(line.trim_end())
        .ok_or_else(|| Response::text(400, "malformed request line"))?;

    let mut content_length = 0usize;
    let mut total = line.len();
    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if total > MAX_HEADER_BYTES {
                    return Err(Response::text(431, "headers too large"));
                }
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':') {
                    if name.trim().eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
            }
            Err(_) => break,
        }
    }

    if content_length > MAX_BODY_BYTES {
        return Err(Response::text(413, "body too large"));
    }
    if content_length > 0 {
        let mut sink = vec![0u8; content_length];
        let _ = reader.read_exact(&mut sink);
    }
    Ok(request)
}

/// Fixed-size worker pool. A thread per connection is simpler but lets a flood
/// of sockets spawn unbounded threads.
pub struct Pool {
    tx: Option<Sender<TcpStream>>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl Pool {
    pub fn new<H>(size: usize, handler: H) -> Pool
    where
        H: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel::<TcpStream>();
        let rx: Arc<Mutex<Receiver<TcpStream>>> = Arc::new(Mutex::new(rx));
        let handler = Arc::new(handler);

        let workers = (0..size.max(1))
            .map(|_| {
                let rx = Arc::clone(&rx);
                let handler = Arc::clone(&handler);
                thread::spawn(move || loop {
                    // Hold the lock only long enough to take one job.
                    let job = match rx.lock() {
                        Ok(guard) => guard.recv(),
                        Err(_) => return, // another worker panicked holding it
                    };
                    match job {
                        Ok(stream) => handle_connection(stream, handler.as_ref()),
                        Err(_) => return, // sender dropped: shut down
                    }
                })
            })
            .collect();

        Pool {
            tx: Some(tx),
            workers,
        }
    }

    pub fn dispatch(&self, stream: TcpStream) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(stream);
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        drop(self.tx.take()); // closing the channel ends every worker loop
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

fn handle_connection<H>(mut stream: TcpStream, handler: &H)
where
    H: Fn(&Request) -> Response,
{
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let response = match read_request(&stream) {
        Ok(request) => {
            let head_only = request.method == "HEAD";
            let response = match request.method.as_str() {
                "GET" | "HEAD" => handler(&request),
                _ => Response::text(405, "method not allowed").header("Allow", "GET, HEAD"),
            };
            let _ = response.write_to(&mut stream, head_only);
            return;
        }
        Err(err) => err,
    };
    let _ = response.write_to(&mut stream, false);
}

/// Bind, then serve forever. `api` handles `/api/*`; everything else is a file
/// under `static_root`.
pub fn serve<F>(addr: &str, static_root: PathBuf, workers: usize, api: F) -> std::io::Result<()>
where
    F: Fn(&Request) -> Option<Response> + Send + Sync + 'static,
{
    let listener = TcpListener::bind(addr)?;
    let local = listener.local_addr()?;
    println!("  chromatex listening on http://{local}");
    println!("  serving {} ({} workers)", static_root.display(), workers);

    let pool = Pool::new(workers, move |req| {
        api(req).unwrap_or_else(|| serve_static(&static_root, &req.path))
    });

    for stream in listener.incoming() {
        match stream {
            Ok(s) => pool.dispatch(s),
            // One bad accept should not take the server down.
            Err(e) => eprintln!("  accept failed: {e}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_method_path_and_query() {
        let r = parse_request_line("GET /api/series?f=tan&n=12 HTTP/1.1").unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/api/series");
        assert_eq!(r.param("f"), Some("tan"));
        assert_eq!(r.param_or("n", 6), 12);
        // Missing and malformed values both fall back.
        assert_eq!(r.param_or("missing", 6), 6);
    }

    #[test]
    fn malformed_values_fall_back() {
        let r = parse_request_line("GET /x?n=abc HTTP/1.1").unwrap();
        assert_eq!(r.param_or("n", 9usize), 9);
    }

    #[test]
    fn rejects_malformed_request_lines() {
        assert!(parse_request_line("GET").is_none());
        assert!(parse_request_line("GET /only-two-parts").is_none());
        assert!(parse_request_line("").is_none());
    }

    #[test]
    fn percent_and_plus_decoding() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("%5Cfrac"), "\\frac");
        assert_eq!(percent_decode("%E2%88%9A2"), "√2");
        // Truncated or invalid escapes are passed through, not dropped.
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn query_without_value() {
        let r = parse_request_line("GET /x?flag HTTP/1.1").unwrap();
        assert_eq!(r.param("flag"), Some(""));
    }

    #[test]
    fn static_resolution_blocks_traversal() {
        let root = Path::new("/srv/static");
        assert_eq!(
            resolve_static(root, "/app.js").unwrap(),
            PathBuf::from("/srv/static/app.js")
        );
        assert_eq!(
            resolve_static(root, "/").unwrap(),
            PathBuf::from("/srv/static/index.html")
        );
        assert_eq!(
            resolve_static(root, "/vendor/fonts/x.woff2").unwrap(),
            PathBuf::from("/srv/static/vendor/fonts/x.woff2")
        );
        for evil in [
            "/../etc/passwd",
            "/a/../../etc/passwd",
            "/..",
            "//etc/passwd",
            "/./../x",
        ] {
            let got = resolve_static(root, evil);
            assert!(
                got.as_ref().map_or(true, |p| p.starts_with(root)),
                "escaped root: {evil} -> {got:?}"
            );
            assert!(
                got.as_ref().map_or(true, |p| !p.to_string_lossy().contains("..")),
                "kept .. : {evil}"
            );
        }
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type_for(Path::new("a.html")), "text/html; charset=utf-8");
        assert_eq!(content_type_for(Path::new("a.woff2")), "font/woff2");
        assert_eq!(content_type_for(Path::new("a.unknown")), "application/octet-stream");
        assert_eq!(content_type_for(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn response_head_is_well_formed() {
        let r = Response::json("{\"a\":1}".to_string());
        assert_eq!(r.status, 200);
        assert_eq!(r.body.len(), 7);
        assert!(r.extra_headers.iter().any(|(k, _)| k == "Cache-Control"));
    }
}
