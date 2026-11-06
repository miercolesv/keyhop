//! Key-hint lighting: one thread owns the keyboard lighting and shows, in
//! priority order, the accent-menu number keys, the terminal's possible next
//! keys, or the user's normal lighting. Running off the event tap means typing
//! never waits on USB.

use crate::accents::{self, Action, Tracker};
use crate::device::Lights;
use crate::keys;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::Instant;

/// How a pattern looks: lit keys, every other key (HSV), and the global
/// brightness to use (None = keep the user's). Keys can't be dimmed one by one
/// (per-key mode uses the global brightness), so contrast comes from colour.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Style {
    lit: [u8; 3],
    rest: [u8; 3],
    brightness: Option<u8>,
}

const WHITE: [u8; 3] = [0, 0, 255];
const BLUE: [u8; 3] = [0xAA, 0xFF, 0xFF];
const ACCENT_STYLE: Style = Style { lit: WHITE, rest: BLUE, brightness: None };
/// Terminal hints stay dim: the whole keyboard drops to a low brightness.
const HINT_STYLE: Style = Style { lit: BLUE, rest: WHITE, brightness: Some(24) };
/// Key presses without a hint update before we assume they go to another app.
const STALE_KEYS: u32 = 3;

pub enum Msg {
    /// A local key event; `plain` = no modifier other than Shift held.
    Key { hid: u16, down: bool, plain: bool },
    /// Keys that may come next in the terminal (empty = none). Sent by the
    /// Unix-socket hint server, so unused on Windows.
    #[cfg_attr(not(unix), allow(dead_code))]
    Hint(Vec<u16>),
}

#[derive(Default)]
struct Want {
    accent: Option<u8>,
    hint: Option<Vec<u16>>,
    keys_since_hint: u32,
}

/// Keys to light and how.
type Pattern = (Vec<u16>, Style);

impl Want {
    /// What to show, or None for the normal lighting.
    fn pattern(&self) -> Option<Pattern> {
        if let Some(n) = self.accent {
            let digits = "1234567890".chars().take(n as usize);
            return Some((digits.filter_map(keys::from_char).collect(), ACCENT_STYLE));
        }
        self.hint.clone().map(|h| (h, HINT_STYLE))
    }
}

/// `accent_menu`: also light the number keys of macOS's accent menu.
pub fn spawn(accent_menu: bool) -> Sender<Msg> {
    let (tx, rx) = channel::<Msg>();
    std::thread::Builder::new()
        .name("key-lights".into())
        .spawn(move || {
            let mut tracker = Tracker::new(accents::choices);
            let mut want = Want::default();
            let mut shown: Option<Pattern> = None;
            let mut lights: Option<Lights> = None;
            let mut first = true;
            loop {
                let msg = match tracker.deadline() {
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                    Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
                };
                let act = match msg {
                    Ok(Msg::Key { hid, down, plain }) => {
                        if std::mem::take(&mut first) {
                            crate::log!("key lights: receiving key events");
                        }
                        if down && keys::mod_bit(hid) == 0 {
                            want.keys_since_hint += 1;
                            if want.keys_since_hint >= STALE_KEYS {
                                want.hint = None;
                            }
                        }
                        if accent_menu {
                            tracker.on_key(hid, down, plain, Instant::now())
                        } else {
                            Action::None
                        }
                    }
                    Ok(Msg::Hint(k)) => {
                        want.keys_since_hint = 0;
                        want.hint = (!k.is_empty()).then_some(k);
                        Action::None
                    }
                    Err(RecvTimeoutError::Timeout) => tracker.on_tick(Instant::now()),
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                match act {
                    Action::Show(n) => want.accent = Some(n),
                    Action::Clear => want.accent = None,
                    Action::None => {}
                }
                let next = want.pattern();
                if next == shown {
                    continue;
                }
                let r = open(&mut lights).and_then(|l| match &next {
                    Some((k, st)) => {
                        let lit: Vec<_> = k.iter().filter_map(|&h| l.led(h)).map(|led| (led, st.lit)).collect();
                        l.show(&lit, st.rest, st.brightness)
                    }
                    None => l.restore(),
                });
                match r {
                    Ok(()) => shown = next,
                    Err(e) => {
                        crate::log!("key lights: {e}");
                        lights = None; // reopen next time (keyboard may have been replugged)
                        shown = None;
                    }
                }
            }
            if let Some(l) = lights.as_mut() {
                let _ = l.restore();
            }
        })
        .expect("spawn key-lights thread");
    tx
}

fn open(l: &mut Option<Lights>) -> Result<&mut Lights, String> {
    if l.is_none() {
        *l = Some(Lights::open()?);
    }
    Ok(l.as_mut().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_menu_wins_over_hint() {
        let mut w = Want { hint: Some(vec![0x04]), ..Default::default() };
        assert_eq!(w.pattern(), Some((vec![0x04], HINT_STYLE)));
        w.accent = Some(2);
        assert_eq!(w.pattern(), Some((vec![0x1E, 0x1F], ACCENT_STYLE)));
        w.accent = None;
        w.hint = None;
        assert_eq!(w.pattern(), None);
    }
}
