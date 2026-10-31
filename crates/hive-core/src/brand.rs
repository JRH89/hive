//! Product identity. Keep user-visible naming in one place.

pub const NAME: &str = "HIVE";
pub const TAGLINE: &str = "One terminal. Many agents.";
/// Plain Unicode (no Nerd Font required). `<>` is the ASCII fallback.
pub const MARK: &str = "◇";
pub const MARK_ASCII: &str = "<>";
/// The hexagon mark: one honeycomb cell holding a prompt. Plain ASCII so it
/// renders in any terminal and font.
pub const MARK_ART: [&str; 5] = [
    r"   ____   ",
    r"  /    \  ",
    r" /  >_  \ ",
    r" \      / ",
    r"  \____/  ",
];
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
