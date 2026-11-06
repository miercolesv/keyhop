//! Accent-menu detection (macOS): decides when a held letter has opened the
//! press-and-hold accent menu and how many number keys pick an accent.
//! `lights.rs` turns that into keyboard lighting.

use std::time::{Duration, Instant};

/// How long a letter must be held before the menu counts as open.
const HOLD: Duration = Duration::from_millis(500);
/// Give up and restore the lighting if no key follows the menu.
const MAX_OPEN: Duration = Duration::from_secs(10);

/// Number of accent choices macOS offers per letter (HID usage, count), as
/// counted in the press-and-hold menu (macOS, 2026-11-06). Other input
/// languages may offer different counts.
const CHOICES: &[(u16, u8)] = &[
    (0x04, 8), // a
    (0x06, 1), // c
    (0x08, 7), // e
    (0x0C, 6), // i
    (0x0D, 1), // j
    (0x11, 2), // n
    (0x12, 8), // o
    (0x18, 5), // u
];

pub fn choices(hid: u16) -> u8 {
    CHOICES.iter().find(|c| c.0 == hid).map_or(0, |c| c.1)
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Show(u8),
    Clear,
}

/// Decides when the menu is open, independent of timing sources and hardware.
pub struct Tracker {
    count: fn(u16) -> u8,
    held: Option<(u16, u8, Instant)>,
    open_since: Option<Instant>,
}

impl Tracker {
    pub fn new(count: fn(u16) -> u8) -> Tracker {
        Tracker { count, held: None, open_since: None }
    }

    /// When `on_tick` should next be called, if anything is pending.
    pub fn deadline(&self) -> Option<Instant> {
        match (self.held, self.open_since) {
            (_, Some(t)) => Some(t + MAX_OPEN),
            (Some((_, _, t)), None) => Some(t + HOLD),
            _ => None,
        }
    }

    /// `plain`: no modifier other than Shift is held.
    pub fn on_key(&mut self, hid: u16, down: bool, plain: bool, now: Instant) -> Action {
        if !down {
            // Releasing the letter keeps an open menu open; before that, it cancels.
            if self.held.is_some_and(|h| h.0 == hid) && self.open_since.is_none() {
                self.held = None;
            }
            return Action::None;
        }
        let was_open = self.open_since.take().is_some();
        self.held = None;
        let n = (self.count)(hid);
        if plain && n > 0 && !was_open {
            self.held = Some((hid, n, now));
        }
        if was_open {
            Action::Clear
        } else {
            Action::None
        }
    }

    pub fn on_tick(&mut self, now: Instant) -> Action {
        if let Some(t) = self.open_since {
            if now >= t + MAX_OPEN {
                self.open_since = None;
                return Action::Clear;
            }
        } else if let Some((_, n, t)) = self.held {
            if now >= t + HOLD {
                self.held = None;
                self.open_since = Some(now);
                return Action::Show(n);
            }
        }
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: u16 = 0x08;
    const X: u16 = 0x1B;
    fn count(hid: u16) -> u8 {
        if hid == E {
            7
        } else {
            0
        }
    }

    #[test]
    fn hold_opens_and_next_key_clears() {
        let t0 = Instant::now();
        let mut t = Tracker::new(count);
        assert_eq!(t.on_key(E, true, true, t0), Action::None);
        assert_eq!(t.on_tick(t0 + HOLD), Action::Show(7));
        assert_eq!(t.on_key(E, false, true, t0 + HOLD * 2), Action::None); // menu stays open
        assert_eq!(t.on_key(0x1F, true, true, t0 + HOLD * 3), Action::Clear); // the "2" key picks
        assert_eq!(t.deadline(), None);
    }

    #[test]
    fn quick_taps_modifiers_and_other_keys_do_nothing() {
        let t0 = Instant::now();
        let mut t = Tracker::new(count);
        t.on_key(E, true, true, t0);
        t.on_key(E, false, true, t0 + HOLD / 2);
        assert_eq!(t.on_tick(t0 + HOLD), Action::None);
        t.on_key(E, true, false, t0); // Cmd+E
        assert_eq!(t.deadline(), None);
        t.on_key(X, true, true, t0); // no accents on X
        assert_eq!(t.deadline(), None);
    }

    #[test]
    fn open_menu_times_out() {
        let t0 = Instant::now();
        let mut t = Tracker::new(count);
        t.on_key(E, true, true, t0);
        assert_eq!(t.on_tick(t0 + HOLD), Action::Show(7));
        assert_eq!(t.on_tick(t0 + HOLD + MAX_OPEN), Action::Clear);
    }
}
