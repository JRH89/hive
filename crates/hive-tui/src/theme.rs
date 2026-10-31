//! Theme tokens and glyphs.
//!
//! Palettes are defined in RGB and downgraded to what the terminal supports:
//! truecolor, the nearest xterm-256 index, the nearest of the 16 ANSI colours,
//! or no colour at all (emphasis then comes from bold and reverse video).
//! Glyphs fall back to ASCII where Unicode is unlikely to render (no UTF-8
//! locale, or the Linux console). No Nerd Font glyphs are ever required.

use hive_core::brand;
use hive_core::config::ThemeConfig;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;

/// Built-in base palettes, selected by `general.theme`.
pub const THEMES: [&str; 4] = ["hive-dark", "hive-light", "ansi", "mono"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    TrueColor,
    Ansi256,
    Ansi16,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct Glyphs {
    pub mark: &'static str,
    pub running: &'static str,
    pub stopped: &'static str,
    pub failed: &'static str,
    pub lost: &'static str,
    /// Left edge of the attached session's sidebar row.
    pub current: &'static str,
    pub group: &'static str,
    /// Text-input caret.
    pub caret: &'static str,
    pub ellipsis: &'static str,
    /// Hive's own UI is folded to ASCII after drawing.
    pub ascii: bool,
    pub border: border::Set<'static>,
}

impl Glyphs {
    pub const UNICODE: Glyphs = Glyphs {
        mark: brand::MARK,
        running: "●",
        stopped: "○",
        failed: "✕",
        lost: "◌",
        current: "▌",
        group: "▾",
        caret: "▏",
        ellipsis: "…",
        ascii: false,
        border: border::ROUNDED,
    };

    pub const ASCII: Glyphs = Glyphs {
        mark: brand::MARK_ASCII,
        running: "*",
        stopped: "o",
        failed: "x",
        lost: "?",
        current: ">",
        group: "-",
        caret: "_",
        ellipsis: "~",
        ascii: true,
        border: border::Set {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        },
    };
}

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
    /// Text drawn on an accent background.
    pub on_accent: Color,
    /// No colours: highlights use reverse video instead of backgrounds.
    pub mono: bool,
    pub glyphs: Glyphs,
}

type Rgb = (u8, u8, u8);

/// Tokens in RGB, before downgrading.
#[derive(Debug, Clone, Copy)]
struct Palette {
    surface: Rgb,
    accent: Rgb,
    accent_dim: Rgb,
    text: Rgb,
    muted: Rgb,
    success: Rgb,
    warning: Rgb,
    error: Rgb,
    on_accent: Rgb,
}

const HIVE_DARK: Palette = Palette {
    surface: (0x1c, 0x20, 0x28),
    accent: (0xf2, 0xb8, 0x4b),
    accent_dim: (0xd9, 0x9a, 0x2b),
    text: (0xe8, 0xea, 0xf0),
    muted: (0x89, 0x92, 0xa3),
    success: (0x79, 0xc9, 0x9e),
    warning: (0xe8, 0xae, 0x55),
    error: (0xe5, 0x73, 0x73),
    on_accent: (0x11, 0x13, 0x18),
};

/// For light terminal backgrounds: darker amber for contrast on white.
const HIVE_LIGHT: Palette = Palette {
    surface: (0xe4, 0xe7, 0xee),
    accent: (0xb7, 0x79, 0x1f),
    accent_dim: (0x8a, 0x5a, 0x12),
    text: (0x1c, 0x20, 0x28),
    muted: (0x5c, 0x64, 0x75),
    success: (0x2f, 0x85, 0x5a),
    warning: (0xa8, 0x6a, 0x10),
    error: (0xc5, 0x30, 0x30),
    on_accent: (0xff, 0xff, 0xff),
};

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("unknown theme `{0}`; built-in themes: {list}", list = THEMES.join(", "))]
    UnknownTheme(String),
    #[error("[theme] {key} = `{value}` is not a #rrggbb colour")]
    BadColor { key: &'static str, value: String },
    #[error("[theme] {key} = `{value}`; expected one of: {expected}")]
    BadChoice {
        key: &'static str,
        value: String,
        expected: &'static str,
    },
}

/// What the environment says about colour and Unicode support.
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub no_color: bool,
    pub colorterm: Option<String>,
    pub term: Option<String>,
    /// First of `LC_ALL`, `LC_CTYPE`, `LANG` that is set.
    pub locale: Option<String>,
}

impl Env {
    pub fn current() -> Env {
        let var = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
        Env {
            no_color: var("NO_COLOR").is_some(),
            colorterm: var("COLORTERM"),
            term: var("TERM"),
            locale: var("LC_ALL")
                .or_else(|| var("LC_CTYPE"))
                .or_else(|| var("LANG")),
        }
    }

    fn depth(&self) -> Depth {
        let term = self.term.as_deref().unwrap_or("");
        if self.no_color || term == "dumb" {
            Depth::None
        } else if self
            .colorterm
            .as_deref()
            .is_some_and(|c| c.contains("truecolor") || c.contains("24bit"))
        {
            Depth::TrueColor
        } else if term.contains("256color") || term.contains("direct") {
            Depth::Ansi256
        } else {
            Depth::Ansi16
        }
    }

    fn unicode(&self) -> bool {
        if self.term.as_deref() == Some("linux") {
            // The VT console's font lacks most of these glyphs.
            return false;
        }
        match &self.locale {
            Some(l) => {
                let l = l.to_ascii_lowercase();
                l.contains("utf-8") || l.contains("utf8")
            }
            None => true,
        }
    }
}

impl Theme {
    #[cfg(test)]
    pub fn hive_dark(truecolor: bool) -> Theme {
        let depth = if truecolor {
            Depth::TrueColor
        } else {
            Depth::Ansi256
        };
        Theme::build(&HIVE_DARK, depth, Glyphs::UNICODE)
    }

    pub fn from_config(name: &str, cfg: &ThemeConfig, env: &Env) -> Result<Theme, ThemeError> {
        let choice =
            |key, value: &Option<String>, options: &[&str], expected| match value.as_deref() {
                None | Some("auto") => Ok(None),
                Some(v) if options.contains(&v) => Ok(Some(v.to_string())),
                Some(v) => Err(ThemeError::BadChoice {
                    key,
                    value: v.to_string(),
                    expected,
                }),
            };
        let mut depth = match choice(
            "colors",
            &cfg.colors,
            &["truecolor", "256", "16", "none"],
            "auto, truecolor, 256, 16, none",
        )?
        .as_deref()
        {
            Some("truecolor") => Depth::TrueColor,
            Some("256") => Depth::Ansi256,
            Some("16") => Depth::Ansi16,
            Some(_) => Depth::None,
            None => env.depth(),
        };
        let glyphs = match choice(
            "glyphs",
            &cfg.glyphs,
            &["unicode", "ascii"],
            "auto, unicode, ascii",
        )?
        .as_deref()
        {
            Some("unicode") => Glyphs::UNICODE,
            Some(_) => Glyphs::ASCII,
            None if env.unicode() => Glyphs::UNICODE,
            None => Glyphs::ASCII,
        };
        let mut palette = match name {
            "hive-dark" => HIVE_DARK,
            "hive-light" => HIVE_LIGHT,
            "ansi" => {
                depth = depth.min_with(Depth::Ansi16);
                HIVE_DARK
            }
            "mono" => {
                depth = Depth::None;
                HIVE_DARK
            }
            other => return Err(ThemeError::UnknownTheme(other.to_string())),
        };
        let overrides: [(&'static str, &Option<String>, &mut Rgb); 8] = [
            ("surface", &cfg.surface, &mut palette.surface),
            ("accent", &cfg.accent, &mut palette.accent),
            ("accent_dim", &cfg.accent_dim, &mut palette.accent_dim),
            ("text", &cfg.text, &mut palette.text),
            ("muted", &cfg.muted, &mut palette.muted),
            ("success", &cfg.success, &mut palette.success),
            ("warning", &cfg.warning, &mut palette.warning),
            ("error", &cfg.error, &mut palette.error),
        ];
        for (key, value, slot) in overrides {
            if let Some(v) = value {
                *slot = parse_hex(v).ok_or_else(|| ThemeError::BadColor {
                    key,
                    value: v.clone(),
                })?;
            }
        }
        Ok(Theme::build(&palette, depth, glyphs))
    }

    fn build(p: &Palette, depth: Depth, glyphs: Glyphs) -> Theme {
        let c = |rgb: Rgb| downgrade(rgb, depth);
        Theme {
            surface: c(p.surface),
            accent: c(p.accent),
            accent_dim: c(p.accent_dim),
            text: c(p.text),
            muted: c(p.muted),
            success: c(p.success),
            warning: c(p.warning),
            error: c(p.error),
            on_accent: c(p.on_accent),
            mono: depth == Depth::None,
            glyphs,
        }
    }

    /// Header and status bar background.
    pub fn bar(&self) -> Style {
        if self.mono {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().bg(self.surface).fg(self.text)
        }
    }

    /// The chosen row in a picker, and the leader badge.
    pub fn highlight(&self) -> Style {
        if self.mono {
            Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::new().bg(self.accent).fg(self.on_accent)
        }
    }

    /// The sidebar row under the cursor.
    pub fn cursor_row(&self) -> Style {
        if self.mono {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().bg(self.surface)
        }
    }
}

impl Depth {
    fn min_with(self, cap: Depth) -> Depth {
        let rank = |d| match d {
            Depth::TrueColor => 3,
            Depth::Ansi256 => 2,
            Depth::Ansi16 => 1,
            Depth::None => 0,
        };
        if rank(self) < rank(cap) { self } else { cap }
    }
}

fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some((byte(0)?, byte(2)?, byte(4)?))
}

fn dist((r1, g1, b1): Rgb, (r2, g2, b2): Rgb) -> u32 {
    let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2) as u32;
    d(r1, r2) + d(g1, g2) + d(b1, b2)
}

fn downgrade(rgb: Rgb, depth: Depth) -> Color {
    match depth {
        Depth::TrueColor => Color::Rgb(rgb.0, rgb.1, rgb.2),
        Depth::Ansi256 => Color::Indexed(nearest_256(rgb)),
        Depth::Ansi16 => nearest_16(rgb),
        Depth::None => Color::Reset,
    }
}

/// Nearest entry of the xterm 6x6x6 cube or grayscale ramp.
fn nearest_256(rgb: Rgb) -> u8 {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let level = |v: u8| {
        (0..6)
            .min_by_key(|&i| (i32::from(LEVELS[i]) - i32::from(v)).abs())
            .unwrap_or(0)
    };
    let (r, g, b) = (level(rgb.0), level(rgb.1), level(rgb.2));
    let cube = (16 + 36 * r + 6 * g + b) as u8;
    let cube_rgb = (LEVELS[r], LEVELS[g], LEVELS[b]);
    let gray = (0..24u8)
        .min_by_key(|&i| dist(rgb, (8 + 10 * i, 8 + 10 * i, 8 + 10 * i)))
        .unwrap_or(0);
    let gray_v = 8 + 10 * gray;
    if dist(rgb, (gray_v, gray_v, gray_v)) < dist(rgb, cube_rgb) {
        232 + gray
    } else {
        cube
    }
}

/// Nearest of the 16 ANSI colours (xterm's default values).
fn nearest_16(rgb: Rgb) -> Color {
    const ANSI: [(Color, Rgb); 16] = [
        (Color::Black, (0, 0, 0)),
        (Color::Red, (205, 0, 0)),
        (Color::Green, (0, 205, 0)),
        (Color::Yellow, (205, 205, 0)),
        (Color::Blue, (0, 0, 238)),
        (Color::Magenta, (205, 0, 205)),
        (Color::Cyan, (0, 205, 205)),
        (Color::Gray, (229, 229, 229)),
        (Color::DarkGray, (127, 127, 127)),
        (Color::LightRed, (255, 0, 0)),
        (Color::LightGreen, (0, 255, 0)),
        (Color::LightYellow, (255, 255, 0)),
        (Color::LightBlue, (92, 92, 255)),
        (Color::LightMagenta, (255, 0, 255)),
        (Color::LightCyan, (0, 255, 255)),
        (Color::White, (255, 255, 255)),
    ];
    ANSI.iter()
        .min_by_key(|(_, c)| dist(rgb, *c))
        .map(|(c, _)| *c)
        .unwrap_or(Color::Reset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(colorterm: Option<&str>, term: &str, locale: Option<&str>) -> Env {
        Env {
            no_color: false,
            colorterm: colorterm.map(str::to_string),
            term: Some(term.to_string()),
            locale: locale.map(str::to_string),
        }
    }

    fn theme(name: &str, cfg: &ThemeConfig, env: &Env) -> Theme {
        Theme::from_config(name, cfg, env).unwrap()
    }

    #[test]
    fn colour_depth_follows_the_terminal() {
        let d = ThemeConfig::default();
        let t = theme(
            "hive-dark",
            &d,
            &env(Some("truecolor"), "xterm-256color", None),
        );
        assert_eq!(t.accent, Color::Rgb(0xf2, 0xb8, 0x4b));
        let t = theme("hive-dark", &d, &env(None, "xterm-256color", None));
        assert!(matches!(t.accent, Color::Indexed(_)));
        let t = theme("hive-dark", &d, &env(None, "xterm", None));
        assert_eq!(t.accent, Color::Yellow);
        let no_color = Env {
            no_color: true,
            ..env(Some("truecolor"), "xterm-256color", None)
        };
        let t = theme("hive-dark", &d, &no_color);
        assert!(t.mono && t.accent == Color::Reset);
    }

    #[test]
    fn named_themes_and_overrides() {
        let tc = env(Some("truecolor"), "xterm-256color", None);
        let cfg = ThemeConfig {
            accent: Some("#112233".into()),
            ..Default::default()
        };
        assert_eq!(
            theme("hive-dark", &cfg, &tc).accent,
            Color::Rgb(0x11, 0x22, 0x33)
        );
        assert!(theme("mono", &ThemeConfig::default(), &tc).mono);
        assert!(matches!(
            theme("ansi", &ThemeConfig::default(), &tc).accent,
            Color::LightYellow | Color::Yellow
        ));
        assert_ne!(
            theme("hive-light", &ThemeConfig::default(), &tc).text,
            theme("hive-dark", &ThemeConfig::default(), &tc).text
        );
        let forced = ThemeConfig {
            colors: Some("16".into()),
            ..Default::default()
        };
        assert!(!matches!(
            theme("hive-dark", &forced, &tc).accent,
            Color::Rgb(..)
        ));
    }

    #[test]
    fn bad_values_are_reported() {
        let e = Env::default();
        assert!(matches!(
            Theme::from_config("neon", &ThemeConfig::default(), &e),
            Err(ThemeError::UnknownTheme(_))
        ));
        let bad = ThemeConfig {
            error: Some("red".into()),
            ..Default::default()
        };
        let err = Theme::from_config("hive-dark", &bad, &e)
            .unwrap_err()
            .to_string();
        assert!(err.contains("error = `red`"), "{err}");
        let bad = ThemeConfig {
            glyphs: Some("emoji".into()),
            ..Default::default()
        };
        assert!(matches!(
            Theme::from_config("hive-dark", &bad, &e),
            Err(ThemeError::BadChoice { .. })
        ));
    }

    #[test]
    fn glyphs_fall_back_to_ascii_without_utf8() {
        let d = ThemeConfig::default();
        let g = |e: Env| theme("hive-dark", &d, &e).glyphs.running;
        assert_eq!(g(env(None, "xterm", Some("en_US.UTF-8"))), "●");
        assert_eq!(g(env(None, "xterm", None)), "●");
        assert_eq!(g(env(None, "xterm", Some("C"))), "*");
        assert_eq!(g(env(None, "linux", Some("en_US.UTF-8"))), "*");
    }

    #[test]
    fn nearest_256_matches_known_entries() {
        assert_eq!(nearest_256((0, 0, 0)), 16);
        assert_eq!(nearest_256((255, 255, 255)), 231);
        assert_eq!(nearest_256((0x1c, 0x20, 0x28)), 235);
        assert_eq!(nearest_256((255, 175, 95)), 215);
    }
}
