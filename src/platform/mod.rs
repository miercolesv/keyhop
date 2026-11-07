//! Platform backends. Each provides the same small API:
//!
//! * `run_capture(handler, grab_mice)` – blocks, feeding every physical key,
//!   button, motion and wheel event to `handler` and honouring its verdict.
//! * `Injector` – synthesises input (key, motion, button, wheel, smooth scroll,
//!   cursor warp) and reports whether the cursor sits on a screen edge.
//! * `set_remote(on)` – stop/start local delivery of physical input.
//! * `screen_bounds()` – union of all displays, if the platform can tell.
//! * `open_raw_hid(vid, usage_page, usage, prefer)` – opens a vendor raw HID interface
//!   as `RawHid` (plus its product ID); `RawHid::transfer` sends one 32-byte
//!   report and returns the reply that echoes it.

#[cfg(target_os = "linux")]
mod hid_linux;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use {hid_linux::*, linux::*};

#[cfg(target_os = "macos")]
mod hid_macos;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use {hid_macos::*, macos::*};

#[cfg(windows)]
mod hid_windows;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use {hid_windows::*, self::windows::*};

/// Marker placed on synthetic events so our own hooks ignore them.
#[allow(dead_code)]
pub const MAGIC: i64 = 0x6B68_6F70; // "khop"

/// A raw HID reply belongs to a request when it echoes the command byte (and the
/// sub-command for the 0xA7 group), or reports the command as unsupported (0xFF).
fn is_reply(out: &[u8; 32], rep: &[u8; 32]) -> bool {
    rep[0] == 0xFF || (rep[0] == out[0] && (out[0] != 0xA7 || rep[1] == out[1]))
}
