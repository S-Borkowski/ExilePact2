//! The quest tracker's only network code: one fixed HTTPS request for the
//! repository's GitHub release list, made through Windows' own WinHTTP.
//!
//! No other crate of the app talks to the network. The public API reaches
//! one place only, [`fetch_release_list`]; the host, port, path and headers
//! are this crate's constants and no caller can pass a URL. The update
//! checker in `qt-app` decides when a check happens and what the answer
//! means; this crate only carries the request.
//!
//! What a request sends, and nothing else:
//!
//! - `GET` [`PATH`] on [`HOST`], port [`PORT`], over TLS ([`SCHEME`]), with
//!   three headers: `Accept: `[`ACCEPT`], `X-GitHub-Api-Version: `
//!   [`API_VERSION`] and `User-Agent: `[`USER_AGENT`] (GitHub refuses a
//!   request without one; it names no version and no machine detail).
//!   GitHub sees the machine's IP address and that header set.
//! - No cookie is kept or sent, no credential is sent (WinHTTP's autologon
//!   policy is set high), and no redirect is followed: a 3xx is returned as
//!   the answer.
//! - The proxy is the system's. TLS and the certificate checks are Windows'
//!   own.
//!
//! Bounds: resolve, connect and send 5 s each, receive 10 s, and the whole
//! request ends within [`TOTAL_DEADLINE`] whatever the server does (the
//! remaining time bounds every wait and is checked between reads). The body
//! is read up to [`MAX_BODY`] bytes and refused past it.
//!
//! Elsewhere than Windows every fetch returns [`FetchError::Unsupported`]
//! without trying.

use std::fmt;
use std::time::Duration;

#[cfg(windows)]
mod win;

/// The only host a request goes to.
pub const HOST: &str = "api.github.com";
/// The only port a request goes to.
pub const PORT: u16 = 443;
/// The only scheme a request uses.
pub const SCHEME: &str = "https";
/// GitHub's REST "List releases" of the repository, the 30 most recent.
pub const PATH: &str = "/repos/S-Borkowski/ExilePact2/releases?per_page=30";
/// The `Accept` header GitHub's REST API documents.
pub const ACCEPT: &str = "application/vnd.github+json";
/// The `X-GitHub-Api-Version` header: the API version the answer is read as.
pub const API_VERSION: &str = "2022-11-28";
/// The `User-Agent` header: the app's name, no version, no machine detail.
pub const USER_AGENT: &str = "ExilePact2-quest-tracker";
/// The largest answer body read; one byte more is refused.
pub const MAX_BODY: usize = 2 * 1024 * 1024;
/// The bound on a whole request, from the first call to the last read.
pub const TOTAL_DEADLINE: Duration = Duration::from_secs(20);

/// The server's answer: its HTTP status and body, whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request produced no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    /// This build has no transport (not Windows); nothing was tried.
    Unsupported,
    /// A wait ran out, or the whole request reached [`TOTAL_DEADLINE`].
    Timeout,
    /// The name did not resolve, or the connection was refused or lost.
    Unreachable,
    /// The TLS connection or the server's certificate failed.
    NotSecure,
    /// The body was longer than [`MAX_BODY`].
    TooLarge,
    /// Any other failure, with WinHTTP's (or the system's) error code.
    Other(u32),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Unsupported => f.write_str("network requests are not supported here"),
            FetchError::Timeout => f.write_str("the request timed out"),
            FetchError::Unreachable => f.write_str("the server could not be reached"),
            FetchError::NotSecure => f.write_str("the secure connection failed"),
            FetchError::TooLarge => f.write_str("the answer was too large"),
            FetchError::Other(code) => write!(f, "the request failed (error {code})"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Fetches the release list: one `GET` of [`PATH`] on [`HOST`], blocking
/// the calling thread for at most [`TOTAL_DEADLINE`]. Any status is an
/// answer (a 3xx, 403 or 500 included); only a failure to get one is an
/// error.
pub fn fetch_release_list() -> Result<Response, FetchError> {
    get(&Target::release_list())
}

/// The bounds of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timeouts {
    pub(crate) resolve: Duration,
    pub(crate) connect: Duration,
    pub(crate) send: Duration,
    pub(crate) receive: Duration,
    pub(crate) total: Duration,
}

/// Where a request goes. Only [`Target::release_list`] is reachable from
/// outside the crate; the tests build their own, plain HTTP to a loopback
/// port they opened, with no proxy and shorter bounds. The path and the
/// headers are never part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) host: &'static str,
    pub(crate) port: u16,
    pub(crate) secure: bool,
    pub(crate) system_proxy: bool,
    pub(crate) timeouts: Timeouts,
}

impl Target {
    pub(crate) fn release_list() -> Self {
        Target {
            host: HOST,
            port: PORT,
            secure: SCHEME == "https",
            system_proxy: true,
            timeouts: Timeouts {
                resolve: Duration::from_secs(5),
                connect: Duration::from_secs(5),
                send: Duration::from_secs(5),
                receive: Duration::from_secs(10),
                total: TOTAL_DEADLINE,
            },
        }
    }
}

#[cfg(windows)]
pub(crate) fn get(target: &Target) -> Result<Response, FetchError> {
    win::get(target)
}

#[cfg(not(windows))]
pub(crate) fn get(_target: &Target) -> Result<Response, FetchError> {
    Err(FetchError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_the_release_list_over_https() {
        assert_eq!(HOST, "api.github.com");
        assert_eq!(PORT, 443);
        assert_eq!(SCHEME, "https");
        assert_eq!(PATH, "/repos/S-Borkowski/ExilePact2/releases?per_page=30");
        assert_eq!(ACCEPT, "application/vnd.github+json");
        assert_eq!(API_VERSION, "2022-11-28");
        assert_eq!(USER_AGENT, "ExilePact2-quest-tracker");
        assert_eq!(MAX_BODY, 2 * 1024 * 1024);
        let t = Target::release_list();
        assert_eq!(t.host, HOST);
        assert_eq!(t.port, PORT);
        assert!(t.secure, "the release list is fetched over TLS only");
        assert!(t.system_proxy);
        assert_eq!(t.timeouts.resolve, Duration::from_secs(5));
        assert_eq!(t.timeouts.connect, Duration::from_secs(5));
        assert_eq!(t.timeouts.send, Duration::from_secs(5));
        assert_eq!(t.timeouts.receive, Duration::from_secs(10));
        assert_eq!(t.timeouts.total, Duration::from_secs(20));
    }

    #[cfg(not(windows))]
    #[test]
    fn fetch_is_unsupported_off_windows() {
        assert_eq!(fetch_release_list(), Err(FetchError::Unsupported));
    }

    /// The loopback tests: a server on `127.0.0.1` (never every interface,
    /// which can raise a firewall prompt) that the test runs, reached
    /// through the same code as the real request.
    #[cfg(windows)]
    mod loopback {
        use super::super::*;
        use std::io::{ErrorKind, Read, Write};
        use std::net::{TcpListener, TcpStream};
        use std::sync::mpsc;
        use std::thread;
        use std::time::Instant;

        fn listen() -> (TcpListener, u16) {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
            let port = listener.local_addr().unwrap().port();
            (listener, port)
        }

        /// Plain HTTP, no proxy, short bounds. Connect stays at 5 s because
        /// Windows retries a refused loopback connect for about 2 s.
        fn target(port: u16) -> Target {
            Target {
                host: "127.0.0.1",
                port,
                secure: false,
                system_proxy: false,
                timeouts: Timeouts {
                    resolve: Duration::from_secs(2),
                    connect: Duration::from_secs(5),
                    send: Duration::from_secs(2),
                    receive: Duration::from_secs(1),
                    total: Duration::from_secs(8),
                },
            }
        }

        /// The server's one connection, or a panic after 10 s, so a client
        /// that never connects fails the test instead of hanging it.
        fn accept(listener: &TcpListener) -> TcpStream {
            listener.set_nonblocking(true).unwrap();
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        return stream;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("no request arrived: {e}"),
                }
            }
        }

        /// Reads one request's head, up to the blank line.
        fn read_head(stream: &mut TcpStream) -> String {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                match stream.read(&mut byte) {
                    Ok(1) => head.push(byte[0]),
                    _ => break,
                }
            }
            String::from_utf8(head).expect("an ASCII request head")
        }

        fn header<'a>(head: &'a str, name: &str) -> Vec<&'a str> {
            head.split("\r\n")
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim())
                .collect()
        }

        #[test]
        fn loopback_get_sends_the_pinned_request_and_returns_the_answer() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                let head = read_head(&mut stream);
                let body = b"[{\"tag_name\":\"quest-tracker-v0.2.0\"}]";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nSet-Cookie: kept=no\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
                head
            });

            let answer = get(&target(port)).expect("an answer");
            let head = server.join().unwrap();

            assert_eq!(answer.status, 200);
            assert_eq!(answer.body, b"[{\"tag_name\":\"quest-tracker-v0.2.0\"}]");
            let request_line = head.split("\r\n").next().unwrap();
            assert_eq!(request_line, format!("GET {PATH} HTTP/1.1"));
            assert_eq!(header(&head, "Accept"), vec![ACCEPT]);
            assert_eq!(header(&head, "X-GitHub-Api-Version"), vec![API_VERSION]);
            assert_eq!(header(&head, "User-Agent"), vec![USER_AGENT]);
            assert_eq!(
                header(&head, "Host"),
                vec![format!("127.0.0.1:{port}").as_str()]
            );
            for absent in ["Cookie", "Authorization", "Proxy-Authorization", "Referer"] {
                assert!(header(&head, absent).is_empty(), "{absent} sent: {head}");
            }
        }

        #[test]
        fn loopback_silent_server_times_out() {
            let (listener, port) = listen();
            let (done, wait) = mpsc::channel::<()>();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                // Holds the connection open and says nothing until the
                // client has given up.
                let _ = wait.recv_timeout(Duration::from_secs(15));
                drop(stream);
            });

            let started = Instant::now();
            let result = get(&target(port));
            let took = started.elapsed();
            done.send(()).unwrap();
            server.join().unwrap();

            // The 1 s receive bound ends it, not the 8 s total; WinHTTP
            // enforces it in steps of about 4 s (measured).
            assert_eq!(result, Err(FetchError::Timeout));
            assert!(took < Duration::from_secs(6), "took {took:?}");
        }

        #[test]
        fn loopback_trickling_server_ends_at_the_total_deadline() {
            let (listener, port) = listen();
            let (done, wait) = mpsc::channel::<()>();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n",
                );
                // One byte every 300 ms: each read is inside the receive
                // bound, so only the total deadline can end it.
                while wait.recv_timeout(Duration::from_millis(300)).is_err() {
                    if stream.write_all(b"x").is_err() {
                        break;
                    }
                }
            });

            let mut short = target(port);
            short.timeouts.total = Duration::from_secs(3);
            let started = Instant::now();
            let result = get(&short);
            let took = started.elapsed();
            let _ = done.send(());
            server.join().unwrap();

            assert_eq!(result, Err(FetchError::Timeout));
            assert!(took < Duration::from_secs(5), "took {took:?}");
        }

        #[test]
        fn loopback_answer_over_the_cap_is_refused() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let len = MAX_BODY + 1;
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
                );
                // The client may close as soon as it is past the cap.
                let _ = stream.write_all(&vec![b'a'; len]);
            });

            let result = get(&target(port));
            server.join().unwrap();

            assert_eq!(result, Err(FetchError::TooLarge));
        }

        #[test]
        fn loopback_answer_at_the_cap_is_read() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {MAX_BODY}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                stream.write_all(&vec![b'a'; MAX_BODY]).unwrap();
            });

            let answer = get(&target(port)).expect("an answer");
            server.join().unwrap();

            assert_eq!(answer.status, 200);
            assert_eq!(answer.body.len(), MAX_BODY);
        }

        #[test]
        fn loopback_redirect_is_not_followed() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 301 Moved Permanently\r\nLocation: http://127.0.0.1:{port}/followed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                drop(stream);
                listener
            });

            let answer = get(&target(port)).expect("the 3xx is the answer");
            let listener = server.join().unwrap();

            assert_eq!(answer.status, 301);
            assert!(answer.body.is_empty());
            // A followed redirect would have connected again by now; a
            // connection the server never accepted still waits in the queue.
            listener.set_nonblocking(true).unwrap();
            match listener.accept() {
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => panic!("accept failed: {e}"),
                Ok((_, from)) => panic!("a second request came from {from}"),
            }
        }

        #[test]
        fn loopback_closed_port_is_unreachable() {
            let (listener, port) = listen();
            drop(listener);

            assert_eq!(get(&target(port)), Err(FetchError::Unreachable));
        }
    }
}
