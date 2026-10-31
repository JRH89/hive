//! Replies to terminal queries emitted by the child process.
//!
//! The daemon owns the authoritative emulator, so it must answer queries such
//! as cursor-position reports itself: no real terminal sees the child's
//! output directly, and many TUIs (Codex, Claude Code) block or misbehave
//! without these replies. Clients' emulators never respond.

use vt100::{Callbacks, Screen};

/// Background/foreground reported for OSC 10/11 colour queries. Matches the
/// hive-dark theme so agents pick their dark palettes.
const OSC_FG: &str = "rgb:e8e8/eaea/f0f0";
const OSC_BG: &str = "rgb:1111/1313/1818";

/// Longest window title or progress payload kept for agent detection.
const MAX_SIGNAL: usize = 256;

#[derive(Default)]
pub(crate) struct Responder {
    pending: Vec<u8>,
    /// Last window title (OSC 0/2), for agent status detection.
    pub(crate) title: String,
    /// Last OSC 9;4 progress payload without the leading `9;`, e.g. `4;1;50`.
    pub(crate) progress: String,
}

impl Responder {
    pub(crate) fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
}

fn capped(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_SIGNAL)]).into_owned()
}

fn first_param(params: &[&[u16]]) -> u16 {
    params.first().and_then(|p| p.first()).copied().unwrap_or(0)
}

impl Callbacks for Responder {
    fn unhandled_csi(
        &mut self,
        screen: &mut Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        match (i1, c) {
            // DSR: device status / cursor position report.
            (None, 'n') => match first_param(params) {
                5 => self.pending.extend_from_slice(b"\x1b[0n"),
                6 => {
                    let (row, col) = screen.cursor_position();
                    self.pending
                        .extend_from_slice(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
                }
                _ => {}
            },
            // Primary DA: claim a VT220 with ANSI colour, like most emulators.
            (None, 'c') if first_param(params) == 0 => {
                self.pending.extend_from_slice(b"\x1b[?62;22c")
            }
            // Secondary DA.
            (Some(b'>'), 'c') if first_param(params) == 0 => {
                self.pending.extend_from_slice(b"\x1b[>0;0;0c")
            }
            _ => {}
        }
    }

    fn set_window_title(&mut self, _screen: &mut Screen, title: &[u8]) {
        self.title = capped(title);
    }

    fn unhandled_osc(&mut self, _screen: &mut Screen, params: &[&[u8]]) {
        // ConEmu-style progress (OSC 9;4;state;percent), which several agents
        // emit while busy.
        if params.first() == Some(&&b"9"[..]) && params.get(1) == Some(&&b"4"[..]) {
            self.progress = capped(&params[1..].join(&b';'));
            return;
        }
        if params.len() == 2 && params[1] == b"?" {
            let color = match params[0] {
                b"10" => OSC_FG,
                b"11" => OSC_BG,
                _ => return,
            };
            let num = std::str::from_utf8(params[0]).unwrap_or_default();
            self.pending
                .extend_from_slice(format!("\x1b]{num};{color}\x1b\\").as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU16;

    fn parser() -> vt100::Parser<Responder> {
        vt100::Parser::new_with_callbacks(
            NonZeroU16::new(24).unwrap(),
            NonZeroU16::new(80).unwrap(),
            0,
            Responder::default(),
        )
    }

    #[test]
    fn cursor_position_report() {
        let mut p = parser();
        p.process(b"\x1b[5;10H\x1b[6n");
        assert_eq!(p.callbacks_mut().take(), b"\x1b[5;10R");
    }

    #[test]
    fn device_attributes() {
        let mut p = parser();
        p.process(b"\x1b[c");
        assert_eq!(p.callbacks_mut().take(), b"\x1b[?62;22c");
        p.process(b"\x1b[>c");
        assert_eq!(p.callbacks_mut().take(), b"\x1b[>0;0;0c");
    }

    #[test]
    fn background_colour_query() {
        let mut p = parser();
        p.process(b"\x1b]11;?\x07");
        assert_eq!(
            p.callbacks_mut().take(),
            format!("\x1b]11;{OSC_BG}\x1b\\").as_bytes()
        );
    }

    #[test]
    fn captures_title_and_progress() {
        let mut p = parser();
        p.process(b"\x1b]0;\xe2\xa0\x8b Claude Code\x07\x1b]9;4;3;\x1b\\");
        assert_eq!(p.callbacks().title, "⠋ Claude Code");
        assert_eq!(p.callbacks().progress, "4;3;");
        p.process(b"\x1b]9;hello\x07");
        assert_eq!(p.callbacks().progress, "4;3;");
        assert!(p.callbacks_mut().take().is_empty());
    }

    #[test]
    fn plain_output_produces_no_reply() {
        let mut p = parser();
        p.process(b"hello \x1b[31mworld\x1b[0m\r\n");
        assert!(p.callbacks_mut().take().is_empty());
    }
}
