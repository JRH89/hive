//! Theme tokens. Truecolor when the terminal advertises it, otherwise the
//! nearest xterm-256 indices, so the UI degrades rather than breaks.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub surface: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub text: Color,
    pub muted: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl Theme {
    pub fn hive_dark(truecolor: bool) -> Theme {
        if truecolor {
            Theme {
                surface: Color::Rgb(0x1c, 0x20, 0x28),
                accent: Color::Rgb(0xf2, 0xb8, 0x4b),
                accent_dim: Color::Rgb(0xd9, 0x9a, 0x2b),
                text: Color::Rgb(0xe8, 0xea, 0xf0),
                muted: Color::Rgb(0x89, 0x92, 0xa3),
                success: Color::Rgb(0x79, 0xc9, 0x9e),
                warning: Color::Rgb(0xe8, 0xae, 0x55),
                error: Color::Rgb(0xe5, 0x73, 0x73),
            }
        } else {
            Theme {
                surface: Color::Indexed(235),
                accent: Color::Indexed(215),
                accent_dim: Color::Indexed(172),
                text: Color::Indexed(255),
                muted: Color::Indexed(245),
                success: Color::Indexed(115),
                warning: Color::Indexed(179),
                error: Color::Indexed(167),
            }
        }
    }

    pub fn detect() -> Theme {
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        Theme::hive_dark(truecolor)
    }
}
