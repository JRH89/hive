//! Product identity. Keep user-visible naming in one place.

pub const NAME: &str = "HIVE";
pub const TAGLINE: &str = "One terminal. Many agents.";
/// Plain Unicode (no Nerd Font required). `<>` is the ASCII fallback.
pub const MARK: &str = "◇";
pub const MARK_ASCII: &str = "<>";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
