//! Raw HID (vendor usage page) access via /dev/hidraw*. Needs read/write access
//! to the node; packaging/99-keyhop-uinput.rules grants it for vendor 3434.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

pub struct RawHid {
    f: File,
}

/// Vendor and product from a hidraw uevent line `HID_ID=0003:00003434:00000C14`.
fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let v = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut it = v.split(':').skip(1).map(|x| u32::from_str_radix(x, 16).ok());
    Some((it.next()?? as u16, it.next()?? as u16))
}

/// Whether a report descriptor declares this top-level usage page and usage.
pub fn descriptor_has_usage(d: &[u8], page: u16, usage: u16) -> bool {
    let (mut cur_page, mut i) = (0u32, 0usize);
    while i < d.len() {
        let pre = d[i];
        if pre == 0xFE {
            // long item
            i += 3 + d.get(i + 1).copied().unwrap_or(0) as usize;
            continue;
        }
        let size = [0, 1, 2, 4][(pre & 3) as usize];
        let mut v = 0u32;
        for k in 0..size {
            v |= (*d.get(i + 1 + k).unwrap_or(&0) as u32) << (8 * k);
        }
        match pre & 0xFC {
            0x04 => cur_page = v,                                                  // Usage Page
            0x08 if cur_page == page as u32 && (v & 0xFFFF) == usage as u32 => return true, // Usage
            _ => {}
        }
        i += 1 + size;
    }
    false
}

/// Open the first hidraw node with this vendor ID and usage; returns it and its product ID.
pub fn open_raw_hid(vid: u16, usage_page: u16, usage: u16) -> Result<(RawHid, u16), String> {
    let dir = std::fs::read_dir("/sys/class/hidraw").map_err(|e| format!("/sys/class/hidraw: {e}"))?;
    let mut denied = None;
    for ent in dir.flatten() {
        let base = ent.path().join("device");
        let Ok(uevent) = std::fs::read_to_string(base.join("uevent")) else { continue };
        let Some((v, pid)) = parse_hid_id(&uevent) else { continue };
        let Ok(desc) = std::fs::read(base.join("report_descriptor")) else { continue };
        if v != vid || !descriptor_has_usage(&desc, usage_page, usage) {
            continue;
        }
        let node = std::path::Path::new("/dev").join(ent.file_name());
        match OpenOptions::new().read(true).write(true).open(&node) {
            Ok(f) => return Ok((RawHid { f }, pid)),
            Err(e) => denied = Some(format!("{}: {e} (install packaging/99-keyhop-uinput.rules)", node.display())),
        }
    }
    Err(denied.unwrap_or_else(|| format!("no keyboard found with vendor 0x{vid:04X} raw HID interface")))
}

impl RawHid {
    /// Send one 32-byte report and wait up to 1 s for the reply that echoes its command bytes.
    pub fn transfer(&mut self, out: &[u8; 32]) -> Result<[u8; 32], String> {
        let mut pkt = [0u8; 33]; // report ID 0, then the report
        pkt[1..].copy_from_slice(out);
        self.f.write_all(&pkt).map_err(|e| format!("HID write: {e}"))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err("keyboard did not answer".into());
            }
            let mut pfd = libc::pollfd { fd: self.f.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            if unsafe { libc::poll(&mut pfd, 1, left.as_millis() as i32) } <= 0 {
                continue;
            }
            let mut rep = [0u8; 32];
            let n = self.f.read(&mut rep).map_err(|e| format!("HID read: {e}"))?;
            if n > 0 && super::is_reply(out, &rep) {
                return Ok(rep);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_vendor_usage() {
        // Usage Page (0xFF60), Usage (0x61), Collection (Application) ...
        let d = [0x06, 0x60, 0xFF, 0x09, 0x61, 0xA1, 0x01, 0xC0];
        assert!(descriptor_has_usage(&d, 0xFF60, 0x61));
        assert!(!descriptor_has_usage(&[0x05, 0x01, 0x09, 0x06], 0xFF60, 0x61));
        assert_eq!(parse_hid_id("DRIVER=hid-generic\nHID_ID=0003:00003434:00000C14\n"), Some((0x3434, 0x0C14)));
    }
}
