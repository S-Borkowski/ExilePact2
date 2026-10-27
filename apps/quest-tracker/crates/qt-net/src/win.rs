//! The request over WinHTTP, synchronous, on a thread of its own.
//!
//! Every handle is closed when it goes out of scope (request, connection,
//! session, in that order). Each blocking call's wait is the smaller of its
//! own bound and what is left of the total deadline, so a server that
//! answers slowly but within each bound still cannot hold the request past
//! the deadline.

use std::ffi::c_void;
use std::iter;
use std::ptr;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::core::BOOL;
use windows_sys::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetOption, WinHttpSetTimeouts, ERROR_WINHTTP_CANNOT_CONNECT,
    ERROR_WINHTTP_CLIENT_AUTH_CERT_NEEDED, ERROR_WINHTTP_CONNECTION_ERROR,
    ERROR_WINHTTP_NAME_NOT_RESOLVED, ERROR_WINHTTP_SECURE_CERT_CN_INVALID,
    ERROR_WINHTTP_SECURE_CERT_DATE_INVALID, ERROR_WINHTTP_SECURE_CERT_REVOKED,
    ERROR_WINHTTP_SECURE_CERT_REV_FAILED, ERROR_WINHTTP_SECURE_CERT_WRONG_USAGE,
    ERROR_WINHTTP_SECURE_CHANNEL_ERROR, ERROR_WINHTTP_SECURE_FAILURE,
    ERROR_WINHTTP_SECURE_INVALID_CA, ERROR_WINHTTP_SECURE_INVALID_CERT, ERROR_WINHTTP_TIMEOUT,
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_NO_PROXY,
    WINHTTP_AUTOLOGON_SECURITY_LEVEL_HIGH, WINHTTP_DISABLE_COOKIES, WINHTTP_DISABLE_REDIRECTS,
    WINHTTP_FLAG_SECURE, WINHTTP_OPTION_AUTOLOGON_POLICY, WINHTTP_OPTION_DISABLE_FEATURE,
    WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT, WINHTTP_OPTION_RECEIVE_TIMEOUT,
    WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_NEVER,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

use crate::{FetchError, Response, Target, ACCEPT, API_VERSION, MAX_BODY, PATH, USER_AGENT};

/// The read buffer: 64 KiB per `WinHttpReadData`.
const CHUNK: usize = 64 * 1024;

/// The request on a thread of its own, waited for until the total deadline.
///
/// WinHTTP enforces a receive bound in coarse steps (measured on Windows 11:
/// a 1 s, 2 s or 3 s bound ended after 4 s, a 6 s one after 8 s), so the
/// last wait inside the request may run a few seconds past the deadline.
/// The caller is answered at the deadline regardless; the thread then ends
/// within its own bounds and its late answer is dropped.
pub(crate) fn get(target: &Target) -> Result<Response, FetchError> {
    let target = *target;
    on_own_thread(target.timeouts.total, move |deadline| {
        request(&target, deadline)
    })
}

/// Runs `work` on a thread of its own and waits for its answer until
/// `total` has passed. A thread that ends without answering (it panicked)
/// is [`FetchError::Panicked`].
fn on_own_thread<F>(total: Duration, work: F) -> Result<Response, FetchError>
where
    F: FnOnce(Deadline) -> Result<Response, FetchError> + Send + 'static,
{
    let deadline = Instant::now() + total;
    let (send, answer) = mpsc::channel();
    thread::Builder::new()
        .name("qt-net request".into())
        .spawn(move || {
            let _ = send.send(work(Deadline(deadline)));
        })
        .map_err(|e| FetchError::Other(e.raw_os_error().unwrap_or(0) as u32))?;
    match answer.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(FetchError::Timeout),
        // The thread ended without an answer: it panicked.
        Err(RecvTimeoutError::Disconnected) => Err(FetchError::Panicked),
    }
}

fn request(target: &Target, deadline: Deadline) -> Result<Response, FetchError> {
    let t = &target.timeouts;

    let agent = wide(USER_AGENT);
    let access = if target.system_proxy {
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY
    } else {
        WINHTTP_ACCESS_TYPE_NO_PROXY
    };
    // SAFETY: `agent` is a NUL-terminated UTF-16 string alive for the call;
    // these access types take no proxy name or bypass list (null), and flags
    // 0 make the session synchronous.
    let session =
        Handle::new(unsafe { WinHttpOpen(agent.as_ptr(), access, ptr::null(), ptr::null(), 0) })?;
    session.set(
        WINHTTP_OPTION_REDIRECT_POLICY,
        WINHTTP_OPTION_REDIRECT_POLICY_NEVER,
    )?;

    let host = wide(target.host);
    // SAFETY: `session` is an open session handle and `host` a NUL-terminated
    // UTF-16 string alive for the call; the reserved argument must be 0.
    let connection =
        Handle::new(unsafe { WinHttpConnect(session.0, host.as_ptr(), target.port, 0) })?;

    let verb = wide("GET");
    let path = wide(PATH);
    let flags = if target.secure {
        WINHTTP_FLAG_SECURE
    } else {
        0
    };
    // SAFETY: `connection` is an open connect handle; `verb` and `path` are
    // NUL-terminated UTF-16 strings alive for the call. Null version, referrer
    // and accept types are the documented defaults (HTTP/1.1, none, none).
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            flags,
        )
    })?;
    request.set(
        WINHTTP_OPTION_DISABLE_FEATURE,
        WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_REDIRECTS,
    )?;
    request.set(
        WINHTTP_OPTION_AUTOLOGON_POLICY,
        WINHTTP_AUTOLOGON_SECURITY_LEVEL_HIGH,
    )?;

    let left = deadline.left()?;
    // SAFETY: `request` is an open request handle; the four values are plain
    // integers.
    check(unsafe {
        WinHttpSetTimeouts(
            request.0,
            millis(t.resolve.min(left)),
            millis(t.connect.min(left)),
            millis(t.send.min(left)),
            millis(t.receive.min(left)),
        )
    })?;
    let headers: Vec<u16> = format!("Accept: {ACCEPT}\r\nX-GitHub-Api-Version: {API_VERSION}\r\n")
        .encode_utf16()
        .collect();
    // SAFETY: `request` is an open request handle; `headers` holds exactly
    // `headers.len()` UTF-16 units and is alive for the call; there is no
    // request body (null, 0, 0) and no context value.
    check(unsafe {
        WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            headers.len() as u32,
            ptr::null(),
            0,
            0,
            0,
        )
    })?;

    let wait = millis(t.receive.min(deadline.left()?)) as u32;
    request.set(WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT, wait)?;
    request.set(WINHTTP_OPTION_RECEIVE_TIMEOUT, wait)?;
    // SAFETY: `request` is an open request handle whose request was sent; the
    // reserved argument must be null.
    check(unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) })?;

    let mut status: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: `request` has received its response; the buffer is `status`,
    // `size` bytes long, as the number flag requires; a null header name and
    // index ask for the status line's code.
    check(unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast::<c_void>(),
            &mut size,
            ptr::null_mut(),
        )
    })?;

    // A synchronous read waits until its whole buffer is filled, so each
    // read asks only for what has already arrived; the wait for more is
    // `WinHttpQueryDataAvailable`'s, and the deadline is checked between.
    let mut body = Vec::new();
    let mut chunk = vec![0u8; CHUNK];
    loop {
        request.set(
            WINHTTP_OPTION_RECEIVE_TIMEOUT,
            millis(t.receive.min(deadline.left()?)) as u32,
        )?;
        let mut available: u32 = 0;
        // SAFETY: `request` has received its response; `available` receives
        // the count of bytes ready to read.
        check(unsafe { WinHttpQueryDataAvailable(request.0, &mut available) })?;
        if available == 0 {
            break;
        }
        let want = (available as usize).min(CHUNK) as u32;
        let mut read: u32 = 0;
        // SAFETY: `request` has received its response; `chunk` is a writable
        // buffer of at least `want` bytes and `read` receives the count
        // written.
        check(unsafe {
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast::<c_void>(),
                want,
                &mut read,
            )
        })?;
        if read == 0 {
            break;
        }
        if body.len() + read as usize > MAX_BODY {
            return Err(FetchError::TooLarge);
        }
        body.extend_from_slice(&chunk[..read as usize]);
    }
    // A status line WinHTTP accepted is three digits.
    Ok(Response {
        status: status as u16,
        body,
    })
}

/// An open WinHTTP handle, closed on drop.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> Result<Self, FetchError> {
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(Handle(raw))
        }
    }

    /// Sets a `u32` option on this handle.
    fn set(&self, option: u32, value: u32) -> Result<(), FetchError> {
        // SAFETY: `self.0` is an open handle; the buffer is `value`, four
        // bytes, alive for the call, as every option set here takes a DWORD.
        check(unsafe {
            WinHttpSetOption(
                self.0,
                option,
                (&value as *const u32).cast::<c_void>(),
                std::mem::size_of::<u32>() as u32,
            )
        })
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a handle WinHTTP returned, closed only here,
        // once; a child handle is dropped before its parent.
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// The end of the whole request.
struct Deadline(Instant);

impl Deadline {
    /// What is left of it, or a timeout once it has passed.
    fn left(&self) -> Result<Duration, FetchError> {
        let left = self.0.saturating_duration_since(Instant::now());
        if left.is_zero() {
            Err(FetchError::Timeout)
        } else {
            Ok(left)
        }
    }
}

fn check(ok: BOOL) -> Result<(), FetchError> {
    if ok != 0 {
        Ok(())
    } else {
        Err(last_error())
    }
}

/// The calling thread's last error, read right after the failed call.
fn last_error() -> FetchError {
    classify(std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32)
}

fn classify(code: u32) -> FetchError {
    match code {
        ERROR_WINHTTP_TIMEOUT => FetchError::Timeout,
        ERROR_WINHTTP_NAME_NOT_RESOLVED
        | ERROR_WINHTTP_CANNOT_CONNECT
        | ERROR_WINHTTP_CONNECTION_ERROR => FetchError::Unreachable,
        ERROR_WINHTTP_SECURE_FAILURE
        | ERROR_WINHTTP_SECURE_CHANNEL_ERROR
        | ERROR_WINHTTP_SECURE_INVALID_CA
        | ERROR_WINHTTP_SECURE_INVALID_CERT
        | ERROR_WINHTTP_SECURE_CERT_CN_INVALID
        | ERROR_WINHTTP_SECURE_CERT_DATE_INVALID
        | ERROR_WINHTTP_SECURE_CERT_REVOKED
        | ERROR_WINHTTP_SECURE_CERT_REV_FAILED
        | ERROR_WINHTTP_SECURE_CERT_WRONG_USAGE
        | ERROR_WINHTTP_CLIENT_AUTH_CERT_NEEDED => FetchError::NotSecure,
        other => FetchError::Other(other),
    }
}

/// A bound in whole milliseconds, at least 1 (WinHTTP reads 0 as "wait
/// forever" for some of them).
fn millis(d: Duration) -> i32 {
    d.as_millis().clamp(1, i32::MAX as u128) as i32
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_thread_that_panics_is_panicked_not_other() {
        let started = Instant::now();
        let result = on_own_thread(Duration::from_secs(5), |_| {
            panic!("the request's own thread panicked (expected by this test)")
        });
        assert_eq!(result, Err(FetchError::Panicked));
        // Answered when the thread ended, not at the deadline.
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(
            FetchError::Panicked.to_string(),
            "the request stopped unexpectedly"
        );
    }

    #[test]
    fn a_request_thread_that_answers_is_its_answer() {
        // Control for the test above: the same runner passes an answer on.
        let result = on_own_thread(Duration::from_secs(5), |_| {
            Ok(Response {
                status: 204,
                body: Vec::new(),
            })
        });
        assert_eq!(
            result,
            Ok(Response {
                status: 204,
                body: Vec::new()
            })
        );
    }
}
