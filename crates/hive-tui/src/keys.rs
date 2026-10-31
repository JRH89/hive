//! Converts crossterm key events back into the byte sequences a terminal
//! would send, so input reaches the agent as if typed directly.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// xterm modifier parameter: 1 + shift + 2*alt + 4*ctrl.
fn modifier_param(mods: KeyModifiers) -> u8 {
    1 + mods.contains(KeyModifiers::SHIFT) as u8
        + 2 * mods.contains(KeyModifiers::ALT) as u8
        + 4 * mods.contains(KeyModifiers::CONTROL) as u8
}

/// Encodes `key` for a PTY. `app_cursor` is DECCKM (application cursor
/// keys), taken from the session's emulator.
pub fn encode(key: &KeyEvent, app_cursor: bool) -> Option<Vec<u8>> {
    let mods = key.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let m =
        modifier_param(mods & (KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL));

    // Cursor/editing keys: CSI 1;m X when modified, SS3/CSI X otherwise.
    let letter = |c: char| -> Vec<u8> {
        if m > 1 {
            format!("\x1b[1;{m}{c}").into_bytes()
        } else if app_cursor {
            format!("\x1bO{c}").into_bytes()
        } else {
            format!("\x1b[{c}").into_bytes()
        }
    };
    // Tilde keys: CSI n ~ or CSI n;m ~.
    let tilde = |n: u8| -> Vec<u8> {
        if m > 1 {
            format!("\x1b[{n};{m}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };

    let bytes = match key.code {
        KeyCode::Char(c) => {
            let mut out = Vec::new();
            if alt {
                out.push(0x1b);
            }
            if ctrl {
                out.push(ctrl_byte(c)?);
            } else {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            out
        }
        KeyCode::Enter => with_alt(alt, b"\r"),
        KeyCode::Tab => with_alt(alt, b"\t"),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => with_alt(alt, if ctrl { b"\x08" } else { b"\x7f" }),
        KeyCode::Esc => with_alt(alt, b"\x1b"),
        KeyCode::Up => letter('A'),
        KeyCode::Down => letter('B'),
        KeyCode::Right => letter('C'),
        KeyCode::Left => letter('D'),
        KeyCode::Home => letter('H'),
        KeyCode::End => letter('F'),
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        KeyCode::F(n @ 1..=4) => {
            let c = b"PQRS"[n as usize - 1] as char;
            if m > 1 {
                format!("\x1b[1;{m}{c}").into_bytes()
            } else {
                format!("\x1bO{c}").into_bytes()
            }
        }
        KeyCode::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][n as usize - 5]),
        _ => return None,
    };
    Some(bytes)
}

fn with_alt(alt: bool, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 1);
    if alt {
        out.push(0x1b);
    }
    out.extend_from_slice(bytes);
    out
}

/// Control character for Ctrl+`c`. crossterm reports 0x1c..=0x1f as
/// Ctrl+'4'..='7', so those are accepted alongside their symbol forms.
fn ctrl_byte(c: char) -> Option<u8> {
    Some(match c.to_ascii_lowercase() {
        c @ 'a'..='z' => c as u8 - b'a' + 1,
        ' ' | '@' | '2' => 0x00,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '7' | '/' => 0x1f,
        '?' | '8' => 0x7f,
        _ => return None,
    })
}

/// Pasted text, wrapped in bracketed-paste markers when the agent enabled
/// them. Without bracketed paste, newlines become CR like a real terminal.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        // Strip any embedded end marker so pasted text cannot escape.
        let clean = text.replace("\x1b[201~", "");
        [b"\x1b[200~", clean.as_bytes(), b"\x1b[201~"].concat()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn printable_and_control_chars() {
        assert_eq!(
            encode(&key(KeyCode::Char('a'), KeyModifiers::NONE), false).unwrap(),
            b"a"
        );
        assert_eq!(
            encode(&key(KeyCode::Char('é'), KeyModifiers::NONE), false).unwrap(),
            "é".as_bytes()
        );
        assert_eq!(
            encode(&key(KeyCode::Char('c'), KeyModifiers::CONTROL), false).unwrap(),
            b"\x03"
        );
        assert_eq!(
            encode(&key(KeyCode::Char(' '), KeyModifiers::CONTROL), false).unwrap(),
            b"\x00"
        );
        assert_eq!(
            encode(&key(KeyCode::Char('x'), KeyModifiers::ALT), false).unwrap(),
            b"\x1bx"
        );
        assert_eq!(
            encode(&key(KeyCode::Char('5'), KeyModifiers::CONTROL), false).unwrap(),
            b"\x1d"
        );
    }

    #[test]
    fn cursor_keys_respect_decckm_and_modifiers() {
        assert_eq!(
            encode(&key(KeyCode::Up, KeyModifiers::NONE), false).unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            encode(&key(KeyCode::Up, KeyModifiers::NONE), true).unwrap(),
            b"\x1bOA"
        );
        assert_eq!(
            encode(&key(KeyCode::Left, KeyModifiers::CONTROL), true).unwrap(),
            b"\x1b[1;5D"
        );
        assert_eq!(
            encode(&key(KeyCode::Delete, KeyModifiers::NONE), false).unwrap(),
            b"\x1b[3~"
        );
        assert_eq!(
            encode(&key(KeyCode::PageUp, KeyModifiers::SHIFT), false).unwrap(),
            b"\x1b[5;2~"
        );
        assert_eq!(
            encode(&key(KeyCode::F(5), KeyModifiers::NONE), false).unwrap(),
            b"\x1b[15~"
        );
    }

    #[test]
    fn editing_keys() {
        assert_eq!(
            encode(&key(KeyCode::Enter, KeyModifiers::NONE), false).unwrap(),
            b"\r"
        );
        assert_eq!(
            encode(&key(KeyCode::Backspace, KeyModifiers::NONE), false).unwrap(),
            b"\x7f"
        );
        assert_eq!(
            encode(&key(KeyCode::BackTab, KeyModifiers::SHIFT), false).unwrap(),
            b"\x1b[Z"
        );
        assert_eq!(
            encode(&key(KeyCode::Enter, KeyModifiers::ALT), false).unwrap(),
            b"\x1b\r"
        );
    }

    #[test]
    fn paste_modes() {
        assert_eq!(encode_paste("a\nb", false), b"a\rb");
        assert_eq!(encode_paste("a\nb", true), b"\x1b[200~a\nb\x1b[201~");
        assert_eq!(encode_paste("x\x1b[201~y", true), b"\x1b[200~xy\x1b[201~");
    }
}
