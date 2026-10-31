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

#[derive(Default)]
pub(crate) struct Responder {
    pending: Vec<u8>,
}

impl Responder {
    pub(crate) fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
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

    fn unhandled_osc(&mut self, _screen: &mut Screen, params: &[&[u8]]) {
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
    fn plain_output_produces_no_reply() {
        let mut p = parser();
        p.process(b"hello \x1b[31mworld\x1b[0m\r\n");
        assert!(p.callbacks_mut().take().is_empty());
    }
}
