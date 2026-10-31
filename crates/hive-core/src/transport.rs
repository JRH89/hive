//! The byte stream between clients and the daemon: a Unix socket (0600,
//! peer uid checked by the daemon), or on Windows a named pipe that a
//! client must open by sending the daemon's token first (ADR-0021).

use std::io;

use crate::paths::Paths;

/// Bytes of the Windows connection token (hex of 32 random bytes).
pub const TOKEN_LEN: usize = 64;

#[cfg(unix)]
pub type Stream = tokio::net::UnixStream;
#[cfg(windows)]
pub type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

pub type ReadHalf = tokio::io::ReadHalf<Stream>;
pub type WriteHalf = tokio::io::WriteHalf<Stream>;

/// Connects to the daemon. `NotFound`/`ConnectionRefused` mean it is not
/// running.
#[cfg(unix)]
pub async fn connect(paths: &Paths) -> io::Result<Stream> {
    tokio::net::UnixStream::connect(&paths.socket).await
}

#[cfg(windows)]
pub async fn connect(paths: &Paths) -> io::Result<Stream> {
    use std::time::{Duration, Instant};
    use tokio::io::AsyncWriteExt;
    use tokio::net::windows::named_pipe::ClientOptions;

    const ERROR_PIPE_BUSY: i32 = 231;
    let name = paths.pipe_name();
    // Busy means every instance is mid-accept; the daemon makes a new one
    // right after each connection.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut pipe = loop {
        match ClientOptions::new().open(&name) {
            Ok(p) => break p,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => return Err(e),
        }
    };
    let token = std::fs::read(paths.token_file())?;
    pipe.write_all(&token).await?;
    Ok(pipe)
}

/// A fresh connection token: hex of 32 bytes from the OS's randomness, by
/// way of std's randomly keyed SipHash (no extra dependency).
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    (0..4)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(i);
            h.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos()),
            );
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Compares in constant time, so a wrong token leaks nothing by timing.
pub fn token_matches(expected: &[u8], got: &[u8]) -> bool {
    expected.len() == got.len()
        && expected
            .iter()
            .zip(got)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_hex_of_the_right_length_and_differ() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), TOKEN_LEN);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn token_comparison() {
        assert!(token_matches(b"abcd", b"abcd"));
        assert!(!token_matches(b"abcd", b"abce"));
        assert!(!token_matches(b"abcd", b"abc"));
    }
}
