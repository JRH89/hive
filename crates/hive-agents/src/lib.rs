//! Which agent runs in a session and what it is doing.
//!
//! - [`catalog`]: agent CLIs Hive knows (launch command, aliases).
//! - [`process`]: recognising an agent from a running process's argv.
//! - [`manifest`]: herdr-compatible screen rules that yield a [`State`].
//! - [`Tracker`]: smooths raw detections into published states.

pub mod catalog;
pub mod manifest;
pub mod process;

use std::time::{Duration, Instant};

pub use catalog::{AgentKind, CATALOG};
pub use manifest::{Detection, Input, Manifest, State};

/// How long a working agent must look plainly idle before it is reported
/// idle. Spinners redraw and briefly vanish between frames; without this a
/// working agent flickers to idle (and would look "done").
pub const IDLE_CONFIRM: Duration = Duration::from_millis(1000);

/// Turns successive [`Detection`]s into a stable state.
#[derive(Debug)]
pub struct Tracker {
    state: State,
    pending_idle: Option<Instant>,
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            state: State::Unknown,
            pending_idle: None,
        }
    }
}

impl Tracker {
    pub fn state(&self) -> State {
        self.state
    }

    /// A working-to-idle change is waiting for confirmation, so the screen
    /// should be checked again even if it hasn't changed.
    pub fn is_settling(&self) -> bool {
        self.pending_idle.is_some()
    }

    /// Feeds one detection; returns the new state if it changed.
    pub fn update(&mut self, d: &Detection, now: Instant) -> Option<State> {
        if d.skip_update {
            return None;
        }
        let plain_idle = self.state == State::Working && d.state == State::Idle && !d.visible;
        if plain_idle {
            let since = *self.pending_idle.get_or_insert(now);
            if now.duration_since(since) < IDLE_CONFIRM {
                return None;
            }
        }
        self.pending_idle = None;
        (d.state != self.state).then(|| {
            self.state = d.state;
            d.state
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(state: State, visible: bool) -> Detection {
        Detection {
            state,
            visible,
            skip_update: false,
            rule: None,
        }
    }

    #[test]
    fn working_to_plain_idle_is_confirmed() {
        let t0 = Instant::now();
        let mut t = Tracker::default();
        assert_eq!(
            t.update(&det(State::Working, true), t0),
            Some(State::Working)
        );
        assert_eq!(t.update(&det(State::Idle, false), t0), None);
        // A spinner frame in between resets the wait.
        assert_eq!(
            t.update(&det(State::Working, true), t0 + IDLE_CONFIRM / 2),
            None
        );
        assert_eq!(t.update(&det(State::Idle, false), t0 + IDLE_CONFIRM), None);
        assert_eq!(
            t.update(&det(State::Idle, false), t0 + IDLE_CONFIRM * 2),
            Some(State::Idle)
        );
    }

    #[test]
    fn visible_idle_and_blocked_publish_at_once() {
        let t0 = Instant::now();
        let mut t = Tracker::default();
        t.update(&det(State::Working, true), t0);
        assert_eq!(t.update(&det(State::Idle, true), t0), Some(State::Idle));
        t.update(&det(State::Working, true), t0);
        assert_eq!(
            t.update(&det(State::Blocked, true), t0),
            Some(State::Blocked)
        );
        let skip = Detection {
            skip_update: true,
            ..det(State::Unknown, false)
        };
        assert_eq!(t.update(&skip, t0), None);
        assert_eq!(t.state(), State::Blocked);
    }
}
