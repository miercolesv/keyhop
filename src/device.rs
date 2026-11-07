//! `keyhop device`: read and change keyboard settings over the vendor raw HID
//! interface (usage page 0xFF60, 32-byte reports). The command set was mapped
//! from the V1 Ultra 8K firmware; see docs/hardware/keychron-v1-ultra-8k.md. Only commands whose
//! handlers were checked in that firmware are sent, and writes are limited to
//! firmware we have verified.

use crate::{keys, platform};

const VID: u16 = 0x3434;
const USAGE_PAGE: u16 = 0xFF60;
const USAGE: u16 = 0x61;
/// Product IDs whose firmware handlers were verified; writes are refused elsewhere.
const VERIFIED_PIDS: &[u16] = &[0x0C14];

const KEY_GET: u8 = 0x04;
const KEY_SET: u8 = 0x05;
const FW_VERSION: u8 = 0xA1;
const MISC: u8 = 0xA7;
const DEBOUNCE_GET: u8 = 0x05;
const DEBOUNCE_SET: u8 = 0x06;
const SNAP_SIZE: u8 = 0x07;
const SNAP_GET: u8 = 0x08;
const SNAP_SET: u8 = 0x09;
const SNAP_SAVE: u8 = 0x0A;
const SLEEP_GET: u8 = 0x0B;
const SLEEP_SET: u8 = 0x0C;
const RATE_GET: u8 = 0x0D;
const RATE_SET: u8 = 0x0E;

// Lighting. All of these change keyboard RAM only; keyhop never sends the
// save commands (VIA 0x09, RGB 0x02), so a replug restores the saved lighting.
const VIA_SET: u8 = 0x07;
const VIA_GET: u8 = 0x08;
const VIA_SAVE: u8 = 0x09;
const CH_RGB_MATRIX: u8 = 3;
const VAL_BRIGHTNESS: u8 = 1;
const VAL_EFFECT: u8 = 2;
const VAL_SPEED: u8 = 3;
const EFFECT_PER_KEY: u8 = 23;
const RGB: u8 = 0xA8;
const RGB_LED_COUNT: u8 = 0x05;
const RGB_LED_BY_ROW: u8 = 0x06;
const RGB_GET_COLOR: u8 = 0x09;
const RGB_SET_COLOR: u8 = 0x0A;
const COLORS_PER_REPORT: usize = 9;

/// Polling rate for an index: 8000 Hz >> index, indexes 0..=6.
pub fn rate_hz(idx: u8) -> u32 {
    8000 >> idx.min(6)
}
pub fn rate_index(hz: u32) -> Option<u8> {
    (0..=6u8).find(|&i| rate_hz(i) == hz)
}

pub fn cmd(bytes: &[u8]) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[..bytes.len()].copy_from_slice(bytes);
    b
}

#[derive(Debug, PartialEq)]
pub struct Rate {
    pub wired: u8,
    pub wireless: u8,
}
pub fn parse_rate(r: &[u8; 32]) -> Rate {
    Rate { wired: r[6] & 0x7F, wireless: r[11] }
}

#[derive(Debug, PartialEq)]
pub struct Sleep {
    pub backlight_s: u16,
    pub sleep_s: u16,
}
pub fn parse_sleep(r: &[u8; 32]) -> Sleep {
    Sleep { backlight_s: u16::from_le_bytes([r[3], r[4]]), sleep_s: u16::from_le_bytes([r[5], r[6]]) }
}
pub fn sleep_cmd(s: &Sleep) -> Result<[u8; 32], String> {
    if s.backlight_s < 5 || s.sleep_s < 60 {
        return Err("firmware requires backlight >= 5 s and sleep >= 60 s".into());
    }
    let [b0, b1] = s.backlight_s.to_le_bytes();
    let [s0, s1] = s.sleep_s.to_le_bytes();
    Ok(cmd(&[MISC, SLEEP_SET, b0, b1, s0, s1]))
}

#[derive(Debug, PartialEq)]
pub struct Debounce {
    pub kind: u8,
    pub press: u8,
    pub release: u8,
}
pub fn parse_debounce(r: &[u8; 32]) -> Debounce {
    Debounce { kind: r[3], press: r[4], release: r[5] }
}
pub fn debounce_cmd(press: u8, release: u8) -> Result<[u8; 32], String> {
    if press > 50 || !(1..=50).contains(&release) {
        return Err("firmware accepts press 0-50 and release 1-50".into());
    }
    Ok(cmd(&[MISC, DEBOUNCE_SET, press, release]))
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Snap {
    pub kind: u8,
    pub a: u8,
    pub b: u8,
}
pub fn parse_snaps(r: &[u8; 32], n: usize) -> Vec<Snap> {
    (0..n.min(9)).map(|i| Snap { kind: r[3 + 3 * i], a: r[4 + 3 * i], b: r[5 + 3 * i] }).collect()
}
pub fn snap_cmd(slot: u8, s: Snap) -> [u8; 32] {
    cmd(&[MISC, SNAP_SET, slot, 1, s.kind, s.a, s.b])
}

/// Snap-click resolution types, as named by Keychron Launcher.
const SNAP_TYPES: &[(u8, &str, &str)] = &[
    (2, "last", "last pressed key wins"),
    (3, "first", "first key of the pair always wins"),
    (4, "second", "second key of the pair always wins"),
    (5, "cancel", "both keys are cancelled while held together"),
];
fn snap_type(name: &str) -> Result<u8, String> {
    SNAP_TYPES.iter().find(|t| t.1 == name).map(|t| t.0).ok_or_else(|| {
        let names: Vec<_> = SNAP_TYPES.iter().map(|t| format!("{} ({})", t.1, t.2)).collect();
        format!("snap type must be one of: {}", names.join(", "))
    })
}
fn snap_type_name(kind: u8) -> String {
    SNAP_TYPES.iter().find(|t| t.0 == kind).map_or(format!("type {kind}"), |t| t.1.to_string())
}

/// Snap-click key bytes are keycodes; basic ones equal HID keyboard usages.
fn snap_key(name: &str) -> Result<u8, String> {
    match keys::from_name(name) {
        Some(h) if (0x04..=0xE7).contains(&h) => Ok(h as u8),
        _ => Err(format!("'{name}' is not a basic keyboard key")),
    }
}

/// The firmware stores a brightness write `x` as `x * 255 >> 8`, so writing back
/// a value we read would lose a step each time. Returns the write that stores
/// `v` (255 can't be reached; it becomes 254).
fn brightness_arg(v: u8) -> u8 {
    ((v as u32 * 256).div_ceil(255)).min(255) as u8
}

/// A keycode as hex (`0x2129`, QMK numbering) or a basic key name (`esc`).
fn keycode(s: &str) -> Result<u16, String> {
    if let Some(h) = s.strip_prefix("0x") {
        return u16::from_str_radix(h, 16).map_err(|_| format!("bad keycode '{s}'"));
    }
    keys::from_name(s).filter(|&h| h <= 0xFF).ok_or_else(|| format!("unknown key '{s}' (use a name or 0x hex)"))
}

/// Every reply echoes the command; status byte 2 is 0 on success for setters.
fn expect_ok(r: &[u8; 32], what: &str) -> Result<(), String> {
    if r[0] == 0xFF {
        return Err(format!("{what}: command not supported by this firmware"));
    }
    if r[2] != 0 {
        return Err(format!("{what}: keyboard rejected the value (status {})", r[2]));
    }
    Ok(())
}

/// Product ID of a connected keyboard whose firmware keyhop has verified.
pub fn supported_keyboard() -> Option<u16> {
    platform::open_raw_hid(VID, USAGE_PAGE, USAGE, VERIFIED_PIDS).ok().map(|(_, pid)| pid).filter(|p| VERIFIED_PIDS.contains(p))
}

struct Kbd {
    hid: platform::RawHid,
    pid: u16,
}

impl Kbd {
    fn open() -> Result<Kbd, String> {
        let (hid, pid) = platform::open_raw_hid(VID, USAGE_PAGE, USAGE, VERIFIED_PIDS)?;
        Ok(Kbd { hid, pid })
    }
    fn ask(&mut self, out: [u8; 32]) -> Result<[u8; 32], String> {
        self.hid.transfer(&out)
    }
    fn misc(&mut self, sub: &[u8]) -> Result<[u8; 32], String> {
        let mut b = vec![MISC];
        b.extend_from_slice(sub);
        self.ask(cmd(&b))
    }
    fn writable(&self) -> Result<(), String> {
        if VERIFIED_PIDS.contains(&self.pid) {
            Ok(())
        } else {
            Err(format!("refusing to write: firmware of PID 0x{:04X} has not been verified", self.pid))
        }
    }
}

/// Matrix of the V1 Ultra 8K (the only firmware writes are allowed on).
const ROWS: u8 = 6;
const COLS: usize = 16;
const KEYMAP_GET_BUFFER: u8 = 0x12;

/// Temporarily replaces the keyboard lighting with a per-key pattern, in RAM only.
pub struct Lights {
    k: Kbd,
    leds: usize,
    /// (HID usage, LED) for every plain key on base layer 0.
    led_of: Vec<(u16, u8)>,
    /// Effect, brightness and colours to put back, while a pattern is shown.
    saved: Option<Saved>,
}

struct Saved {
    effect: u8,
    brightness: u8,
    colors: Vec<[u8; 3]>,
}

impl Lights {
    pub fn open() -> Result<Lights, String> {
        let mut k = Kbd::open()?;
        k.writable()?;
        let leds = k.ask(cmd(&[RGB, RGB_LED_COUNT]))?[3] as usize;
        // Layer 0 keymap: big-endian keycodes, row-major.
        let mut map = Vec::with_capacity(ROWS as usize * COLS * 2);
        while map.len() < ROWS as usize * COLS * 2 {
            let n = 28.min(ROWS as usize * COLS * 2 - map.len());
            let [hi, lo] = (map.len() as u16).to_be_bytes();
            map.extend_from_slice(&k.ask(cmd(&[KEYMAP_GET_BUFFER, hi, lo, n as u8]))?[4..4 + n]);
        }
        let mut led_of = Vec::new();
        for row in 0..ROWS {
            let r = k.ask(cmd(&[RGB, RGB_LED_BY_ROW, row, 0xFF, 0xFF, 0xFF]))?;
            for col in 0..COLS {
                let i = (row as usize * COLS + col) * 2;
                let code = u16::from_be_bytes([map[i], map[i + 1]]);
                let led = r[3 + col];
                if (0x04..=0xFF).contains(&code) && (led as usize) < leds && !led_of.iter().any(|e: &(u16, u8)| e.0 == code) {
                    led_of.push((code, led));
                }
            }
        }
        Ok(Lights { k, leds, led_of, saved: None })
    }

    /// LED under the key with this HID usage, if it is on the base layer.
    pub fn led(&self, hid: u16) -> Option<u8> {
        self.led_of.iter().find(|e| e.0 == hid).map(|e| e.1)
    }

    fn colors(&mut self) -> Result<Vec<[u8; 3]>, String> {
        let mut out = Vec::with_capacity(self.leds);
        for start in (0..self.leds).step_by(COLORS_PER_REPORT) {
            let n = COLORS_PER_REPORT.min(self.leds - start);
            let r = self.k.ask(cmd(&[RGB, RGB_GET_COLOR, start as u8, n as u8]))?;
            out.extend((0..n).map(|i| [r[3 + 3 * i], r[4 + 3 * i], r[5 + 3 * i]]));
        }
        Ok(out)
    }

    fn set_colors(&mut self, c: &[[u8; 3]]) -> Result<(), String> {
        for (i, chunk) in c.chunks(COLORS_PER_REPORT).enumerate() {
            let mut b = vec![RGB, RGB_SET_COLOR, (i * COLORS_PER_REPORT) as u8, chunk.len() as u8];
            b.extend(chunk.iter().flatten());
            self.k.ask(cmd(&b))?;
        }
        Ok(())
    }

    fn get_value(&mut self, id: u8) -> Result<u8, String> {
        Ok(self.k.ask(cmd(&[VIA_GET, CH_RGB_MATRIX, id]))?[3])
    }

    fn set_value(&mut self, id: u8, v: u8) -> Result<(), String> {
        let v = if id == VAL_BRIGHTNESS { brightness_arg(v) } else { v };
        self.k.ask(cmd(&[VIA_SET, CH_RGB_MATRIX, id, v])).map(drop)
    }

    /// Show `lit` (LED, HSV) and paint every other key `rest`, optionally at a
    /// different global brightness. The firmware's per-key mode ignores each
    /// key's V and uses the global brightness, so keys cannot be dimmed or
    /// switched off individually; contrast must come from colour.
    pub fn show(&mut self, lit: &[(u8, [u8; 3])], rest: [u8; 3], brightness: Option<u8>) -> Result<(), String> {
        if self.saved.is_none() {
            let effect = self.get_value(VAL_EFFECT)?;
            let brightness = self.get_value(VAL_BRIGHTNESS)?;
            let colors = self.colors()?;
            self.saved = Some(Saved { effect, brightness, colors });
        }
        let mut c = vec![rest; self.leds];
        for &(led, hsv) in lit {
            if let Some(slot) = c.get_mut(led as usize) {
                *slot = hsv;
            }
        }
        self.set_colors(&c)?;
        let b = brightness.or(self.saved.as_ref().map(|s| s.brightness)).unwrap_or(255);
        self.set_value(VAL_BRIGHTNESS, b)?;
        self.set_value(VAL_EFFECT, EFFECT_PER_KEY)
    }

    /// Put back the lighting that was active before `show`.
    pub fn restore(&mut self) -> Result<(), String> {
        // Effect first, so the per-key colours are rewritten while hidden.
        if let Some(s) = self.saved.take() {
            self.set_value(VAL_EFFECT, s.effect)?;
            self.set_value(VAL_BRIGHTNESS, s.brightness)?;
            self.set_colors(&s.colors)?;
        }
        Ok(())
    }
}

fn show(k: &mut Kbd) -> Result<(), String> {
    let fw = k.ask(cmd(&[FW_VERSION]))?;
    let end = fw[1..].iter().position(|&c| c == 0).map_or(31, |p| p + 1);
    println!("keyboard   3434:{:04X}  firmware {}", k.pid, String::from_utf8_lossy(&fw[1..end]));
    let e = k.ask(cmd(&[VIA_GET, CH_RGB_MATRIX, VAL_EFFECT]))?[3];
    let sp = k.ask(cmd(&[VIA_GET, CH_RGB_MATRIX, VAL_SPEED]))?[3];
    println!("lighting   effect {e}, speed {sp}");
    let r = parse_rate(&k.misc(&[RATE_GET])?);
    println!("rate       {} Hz wired, {} Hz wireless", rate_hz(r.wired), rate_hz(r.wireless));
    let d = parse_debounce(&k.misc(&[DEBOUNCE_GET])?);
    println!("debounce   press {} release {} (type {})", d.press, d.release, d.kind);
    let s = parse_sleep(&k.misc(&[SLEEP_GET])?);
    println!("sleep      backlight off {} s, sleep {} s", s.backlight_s, s.sleep_s);
    let n = k.misc(&[SNAP_SIZE])?[3] as usize;
    let snaps = parse_snaps(&k.misc(&[SNAP_GET, 0, n.min(9) as u8])?, n);
    for (i, s) in snaps.iter().enumerate() {
        if s.kind as u16 + s.a as u16 + s.b as u16 == 0 {
            println!("snap {}     off", i + 1);
        } else {
            let (a, b) = (keys::name(s.a as u16), keys::name(s.b as u16));
            println!("snap {}     {} + {}  ({})", i + 1, a, b, snap_type_name(s.kind));
        }
    }
    Ok(())
}

pub const USAGE_TEXT: &str = "  keyhop device                 show keyboard settings (Keychron raw HID)
  keyhop device rate <hz>       set wired polling rate: 8000 4000 2000 1000 500 250 125
  keyhop device sleep <s> <s>   backlight-off and sleep timeouts in seconds
  keyhop device debounce <press> <release>
  keyhop device snap <1-5> <last|first|second|cancel> <key> <key>
  keyhop device snap <1-5> off  snap-click (SOCD) pairs, e.g. snap 1 last a d
  keyhop device effect <1-24>   lighting effect, saved (19 reactive, 23 per-key, 24 rainbow)
  keyhop device speed <0-255>   effect speed, saved (faster = shorter, narrower ripples)
  keyhop device key <layer> <row> <col> [code]
                                read or set one key; code is a name or QMK hex,
                                e.g. key 0 3 0 0x2129 (Caps: tap Esc, hold Ctrl)
";

pub fn main(args: &[&str]) -> Result<(), String> {
    let mut k = Kbd::open()?;
    let num = |s: &str| s.parse::<u32>().map_err(|_| format!("'{s}' is not a number"));
    let u8n = |s: &str| num(s).and_then(|v| u8::try_from(v).map_err(|_| format!("{v} is out of range")));
    match args {
        [] => return show(&mut k),
        ["rate", hz] => {
            k.writable()?;
            let idx = rate_index(num(hz)?).ok_or("rate must be 8000, 4000, 2000, 1000, 500, 250 or 125")?;
            let cur = parse_rate(&k.misc(&[RATE_GET])?);
            let r = k.misc(&[RATE_SET, idx, cur.wireless.min(6), 0])?;
            expect_ok(&r, "rate")?;
        }
        ["sleep", b, s] => {
            k.writable()?;
            let to16 = |s: &str| num(s).and_then(|v| u16::try_from(v).map_err(|_| format!("{v} is out of range")));
            let r = k.ask(sleep_cmd(&Sleep { backlight_s: to16(b)?, sleep_s: to16(s)? })?)?;
            expect_ok(&r, "sleep")?;
        }
        ["debounce", p, r] => {
            k.writable()?;
            let r = k.ask(debounce_cmd(u8n(p)?, u8n(r)?)?)?;
            expect_ok(&r, "debounce")?;
        }
        ["snap", slot, rest @ ..] => {
            k.writable()?;
            let n = k.misc(&[SNAP_SIZE])?[3];
            let slot = u8n(slot)?;
            if slot == 0 || slot > n {
                return Err(format!("slot must be 1-{n}"));
            }
            let s = match rest {
                ["off"] => Snap { kind: 0, a: 0, b: 0 },
                [t, a, b] => Snap { kind: snap_type(t)?, a: snap_key(a)?, b: snap_key(b)? },
                _ => return Err(USAGE_TEXT.into()),
            };
            expect_ok(&k.ask(snap_cmd(slot - 1, s))?, "snap")?;
            k.misc(&[SNAP_SAVE])?;
        }
        ["effect", e] => {
            k.writable()?;
            let e = u8n(e)?;
            if !(1..=24).contains(&e) {
                return Err("effect must be 1-24".into());
            }
            k.ask(cmd(&[VIA_SET, CH_RGB_MATRIX, VAL_EFFECT, e]))?;
            k.ask(cmd(&[VIA_SAVE, CH_RGB_MATRIX, VAL_EFFECT]))?;
        }
        ["speed", v] => {
            k.writable()?;
            k.ask(cmd(&[VIA_SET, CH_RGB_MATRIX, VAL_SPEED, u8n(v)?]))?;
            k.ask(cmd(&[VIA_SAVE, CH_RGB_MATRIX, VAL_SPEED]))?;
        }
        ["key", l, r, c, rest @ ..] => {
            let (l, r, c) = (u8n(l)?, u8n(r)?, u8n(c)?);
            let get = |k: &mut Kbd| k.ask(cmd(&[KEY_GET, l, r, c])).map(|b| u16::from_be_bytes([b[4], b[5]]));
            let old = get(&mut k)?;
            match rest {
                [] => println!("layer {l} row {r} col {c}: 0x{old:04X} ({})", keys::name(old)),
                [code] => {
                    k.writable()?;
                    let [hi, lo] = keycode(code)?.to_be_bytes();
                    k.ask(cmd(&[KEY_SET, l, r, c, hi, lo]))?;
                    let new = get(&mut k)?;
                    println!("layer {l} row {r} col {c}: 0x{old:04X} -> 0x{new:04X}  (undo: keyhop device key {l} {r} {c} 0x{old:04X})");
                }
                _ => return Err(format!("usage:\n{USAGE_TEXT}")),
            }
            return Ok(());
        }
        _ => return Err(format!("usage:\n{USAGE_TEXT}")),
    }
    show(&mut k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(bytes: &[u8]) -> [u8; 32] {
        cmd(bytes)
    }

    // Replies captured from a V1 Ultra 8K (firmware v1.0.0-ZK) on 2026-11-06.
    #[test]
    fn parses_live_replies() {
        let r = reply(&[0xa7, 0x0d, 0, 0x40, 0x1f, 0x7f, 0, 0, 0x40, 0x1f, 0x7f, 0, 0, 0x7d, 0, 0x40, 0x40, 1]);
        assert_eq!(parse_rate(&r), Rate { wired: 0, wireless: 0 });
        assert_eq!(rate_hz(0), 8000);
        let s = parse_sleep(&reply(&[0xa7, 0x0b, 0, 0x58, 0x02, 0x20, 0x1c]));
        assert_eq!(s, Sleep { backlight_s: 600, sleep_s: 7200 });
        let d = parse_debounce(&reply(&[0xa7, 0x05, 0, 1, 0, 0x32]));
        assert_eq!(d, Debounce { kind: 1, press: 0, release: 50 });
        assert!(parse_snaps(&reply(&[0xa7, 0x08]), 5).iter().all(|s| s.kind == 0));
    }

    /// Needs the keyboard attached: `cargo test -- --ignored lights_live`.
    /// Lights keys 1-3 for 3 s (RAM only), then checks everything was restored.
    #[test]
    #[ignore]
    fn lights_live() {
        let mut l = Lights::open().unwrap();
        let effect = l.k.ask(cmd(&[VIA_GET, CH_RGB_MATRIX, VAL_EFFECT])).unwrap()[3];
        let before = l.colors().unwrap();
        let lit: Vec<_> = "123".chars().filter_map(keys::from_char).filter_map(|h| l.led(h)).map(|led| (led, [0, 0, 255])).collect();
        assert_eq!(lit.len(), 3);
        let brightness = l.get_value(VAL_BRIGHTNESS).unwrap();
        l.show(&lit, [0xAA, 0xFF, 0xFF], Some(32)).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        l.restore().unwrap();
        assert_eq!(l.k.ask(cmd(&[VIA_GET, CH_RGB_MATRIX, VAL_EFFECT])).unwrap()[3], effect);
        assert_eq!(l.get_value(VAL_BRIGHTNESS).unwrap(), brightness);
        assert_eq!(l.colors().unwrap(), before);
    }

    #[test]
    fn encodes_and_validates() {
        assert_eq!(rate_index(1000), Some(3));
        assert_eq!(rate_index(999), None);
        assert_eq!(&sleep_cmd(&Sleep { backlight_s: 600, sleep_s: 7200 }).unwrap()[..6], &[0xa7, 0x0c, 0x58, 0x02, 0x20, 0x1c]);
        assert!(sleep_cmd(&Sleep { backlight_s: 4, sleep_s: 7200 }).is_err());
        assert!(debounce_cmd(0, 0).is_err());
        assert_eq!(&snap_cmd(1, Snap { kind: 2, a: 0x04, b: 0x07 })[..7], &[0xa7, 0x09, 1, 1, 2, 0x04, 0x07]);
        assert_eq!(snap_key("a"), Ok(0x04));
        assert!(snap_key("volumeup").is_err());
        assert_eq!(snap_type("last"), Ok(2));
        assert_eq!(keycode("0x2129"), Ok(0x2129));
        for v in 0..255u8 {
            assert_eq!((brightness_arg(v) as u32 * 255 >> 8) as u8, v);
        }
        assert_eq!(keycode("capslock"), Ok(0x39));
        assert!(keycode("0xZZ").is_err());
        assert!(snap_type("2").is_err());
    }
}
