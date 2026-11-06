//! Key identity. USB HID usage codes (keyboard page 0x07) are the wire format;
#![allow(dead_code)] // each platform backend uses a different code column
//! each backend maps its native codes to and from them through one table.

pub const MOD_CTRL: u8 = 1;
pub const MOD_SHIFT: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_META: u8 = 8;

const NO: u16 = 0xFFFF;

/// (name, hid, macOS virtual keycode, Linux evdev code, Windows set-1 scancode (0xE0xx = extended))
pub const TABLE: &[(&str, u16, u16, u16, u16)] = &[
    ("a", 0x04, 0x00, 30, 0x1E),
    ("b", 0x05, 0x0B, 48, 0x30),
    ("c", 0x06, 0x08, 46, 0x2E),
    ("d", 0x07, 0x02, 32, 0x20),
    ("e", 0x08, 0x0E, 18, 0x12),
    ("f", 0x09, 0x03, 33, 0x21),
    ("g", 0x0A, 0x05, 34, 0x22),
    ("h", 0x0B, 0x04, 35, 0x23),
    ("i", 0x0C, 0x22, 23, 0x17),
    ("j", 0x0D, 0x26, 36, 0x24),
    ("k", 0x0E, 0x28, 37, 0x25),
    ("l", 0x0F, 0x25, 38, 0x26),
    ("m", 0x10, 0x2E, 50, 0x32),
    ("n", 0x11, 0x2D, 49, 0x31),
    ("o", 0x12, 0x1F, 24, 0x18),
    ("p", 0x13, 0x23, 25, 0x19),
    ("q", 0x14, 0x0C, 16, 0x10),
    ("r", 0x15, 0x0F, 19, 0x13),
    ("s", 0x16, 0x01, 31, 0x1F),
    ("t", 0x17, 0x11, 20, 0x14),
    ("u", 0x18, 0x20, 22, 0x16),
    ("v", 0x19, 0x09, 47, 0x2F),
    ("w", 0x1A, 0x0D, 17, 0x11),
    ("x", 0x1B, 0x07, 45, 0x2D),
    ("y", 0x1C, 0x10, 21, 0x15),
    ("z", 0x1D, 0x06, 44, 0x2C),
    ("1", 0x1E, 0x12, 2, 0x02),
    ("2", 0x1F, 0x13, 3, 0x03),
    ("3", 0x20, 0x14, 4, 0x04),
    ("4", 0x21, 0x15, 5, 0x05),
    ("5", 0x22, 0x17, 6, 0x06),
    ("6", 0x23, 0x16, 7, 0x07),
    ("7", 0x24, 0x1A, 8, 0x08),
    ("8", 0x25, 0x1C, 9, 0x09),
    ("9", 0x26, 0x19, 10, 0x0A),
    ("0", 0x27, 0x1D, 11, 0x0B),
    ("enter", 0x28, 0x24, 28, 0x1C),
    ("esc", 0x29, 0x35, 1, 0x01),
    ("backspace", 0x2A, 0x33, 14, 0x0E),
    ("tab", 0x2B, 0x30, 15, 0x0F),
    ("space", 0x2C, 0x31, 57, 0x39),
    ("minus", 0x2D, 0x1B, 12, 0x0C),
    ("equal", 0x2E, 0x18, 13, 0x0D),
    ("leftbracket", 0x2F, 0x21, 26, 0x1A),
    ("rightbracket", 0x30, 0x1E, 27, 0x1B),
    ("backslash", 0x31, 0x2A, 43, 0x2B),
    ("semicolon", 0x33, 0x29, 39, 0x27),
    ("quote", 0x34, 0x27, 40, 0x28),
    ("grave", 0x35, 0x32, 41, 0x29),
    ("comma", 0x36, 0x2B, 51, 0x33),
    ("period", 0x37, 0x2F, 52, 0x34),
    ("slash", 0x38, 0x2C, 53, 0x35),
    ("capslock", 0x39, 0x39, 58, 0x3A),
    ("f1", 0x3A, 0x7A, 59, 0x3B),
    ("f2", 0x3B, 0x78, 60, 0x3C),
    ("f3", 0x3C, 0x63, 61, 0x3D),
    ("f4", 0x3D, 0x76, 62, 0x3E),
    ("f5", 0x3E, 0x60, 63, 0x3F),
    ("f6", 0x3F, 0x61, 64, 0x40),
    ("f7", 0x40, 0x62, 65, 0x41),
    ("f8", 0x41, 0x64, 66, 0x42),
    ("f9", 0x42, 0x65, 67, 0x43),
    ("f10", 0x43, 0x6D, 68, 0x44),
    ("f11", 0x44, 0x67, 87, 0x57),
    ("f12", 0x45, 0x6F, 88, 0x58),
    ("printscreen", 0x46, NO, 99, 0xE037),
    ("scrolllock", 0x47, NO, 70, 0x46),
    ("pause", 0x48, NO, 119, 0x45),
    ("insert", 0x49, 0x72, 110, 0xE052),
    ("home", 0x4A, 0x73, 102, 0xE047),
    ("pageup", 0x4B, 0x74, 104, 0xE049),
    ("delete", 0x4C, 0x75, 111, 0xE053),
    ("end", 0x4D, 0x77, 107, 0xE04F),
    ("pagedown", 0x4E, 0x79, 109, 0xE051),
    ("right", 0x4F, 0x7C, 106, 0xE04D),
    ("left", 0x50, 0x7B, 105, 0xE04B),
    ("down", 0x51, 0x7D, 108, 0xE050),
    ("up", 0x52, 0x7E, 103, 0xE048),
    ("numlock", 0x53, 0x47, 69, 0xE045),
    ("kp_divide", 0x54, 0x4B, 98, 0xE035),
    ("kp_multiply", 0x55, 0x43, 55, 0x37),
    ("kp_minus", 0x56, 0x4E, 74, 0x4A),
    ("kp_plus", 0x57, 0x45, 78, 0x4E),
    ("kp_enter", 0x58, 0x4C, 96, 0xE01C),
    ("kp_1", 0x59, 0x53, 79, 0x4F),
    ("kp_2", 0x5A, 0x54, 80, 0x50),
    ("kp_3", 0x5B, 0x55, 81, 0x51),
    ("kp_4", 0x5C, 0x56, 75, 0x4B),
    ("kp_5", 0x5D, 0x57, 76, 0x4C),
    ("kp_6", 0x5E, 0x58, 77, 0x4D),
    ("kp_7", 0x5F, 0x59, 71, 0x47),
    ("kp_8", 0x60, 0x5B, 72, 0x48),
    ("kp_9", 0x61, 0x5C, 73, 0x49),
    ("kp_0", 0x62, 0x52, 82, 0x52),
    ("kp_period", 0x63, 0x41, 83, 0x53),
    ("iso_backslash", 0x64, 0x0A, 86, 0x56),
    ("menu", 0x65, 0x6E, 127, 0xE05D),
    ("kp_equal", 0x67, 0x51, 117, 0x59),
    ("f13", 0x68, 0x69, 183, 0x64),
    ("f14", 0x69, 0x6B, 184, 0x65),
    ("f15", 0x6A, 0x71, 185, 0x66),
    ("f16", 0x6B, 0x6A, 186, 0x67),
    ("f17", 0x6C, 0x40, 187, 0x68),
    ("f18", 0x6D, 0x4F, 188, 0x69),
    ("f19", 0x6E, 0x50, 189, 0x6A),
    ("f20", 0x6F, 0x5A, 190, 0x6B),
    ("f21", 0x70, NO, 191, 0x6C),
    ("f22", 0x71, NO, 192, 0x6D),
    ("f23", 0x72, NO, 193, 0x6E),
    ("f24", 0x73, NO, 194, 0x76),
    ("kp_comma", 0x85, 0x5F, 121, 0x7E),
    ("ro", 0x87, 0x5E, 89, 0x73),
    ("kana", 0x88, 0x68, 90, 0x70),
    ("yen", 0x89, 0x5D, 124, 0x7D),
    ("lctrl", 0xE0, 0x3B, 29, 0x1D),
    ("lshift", 0xE1, 0x38, 42, 0x2A),
    ("lalt", 0xE2, 0x3A, 56, 0x38),
    ("lmeta", 0xE3, 0x37, 125, 0xE05B),
    ("rctrl", 0xE4, 0x3E, 97, 0xE01D),
    ("rshift", 0xE5, 0x3C, 54, 0x36),
    ("ralt", 0xE6, 0x3D, 100, 0xE038),
    ("rmeta", 0xE7, 0x36, 126, 0xE05C),
];

/// Dense reverse-lookup tables built at compile time (no runtime allocation).
const fn build<const N: usize>(col: usize) -> [u8; N] {
    // Stores index+1 of TABLE row (0 = none). TABLE has < 255 rows.
    let mut out = [0u8; N];
    let mut i = 0;
    while i < TABLE.len() {
        let r = TABLE[i];
        let code = match col {
            1 => r.1,
            2 => r.2,
            3 => r.3,
            _ => r.4,
        };
        let idx = if col == 4 && code >= 0xE000 { (code & 0xFF) + 0x100 } else { code };
        if code != NO && (idx as usize) < N {
            out[idx as usize] = (i + 1) as u8;
        }
        i += 1;
    }
    out
}
static BY_HID: [u8; 256] = build::<256>(1);
static BY_MAC: [u8; 128] = build::<128>(2);
static BY_EVDEV: [u8; 256] = build::<256>(3);
static BY_WIN: [u8; 512] = build::<512>(4);

#[inline]
fn row(slot: u8) -> Option<&'static (&'static str, u16, u16, u16, u16)> {
    (slot != 0).then(|| &TABLE[slot as usize - 1])
}

#[inline]
fn hid_row(hid: u16) -> Option<&'static (&'static str, u16, u16, u16, u16)> {
    BY_HID.get(hid as usize).and_then(|&s| row(s))
}

/// Consumer-page keys are carried as `CONSUMER | usage` in the same u16 id space.
pub const CONSUMER: u16 = 0x1000;
const NX_NONE: i16 = -1;

/// (name, consumer usage, evdev, macOS NX_KEYTYPE (system-defined event) or -1,
///  macOS keycode (for keys macOS reports as plain key events) or NO, Windows VK or 0)
/// The first ten are what the Keychron V1 Ultra 8K (PID 0x0C14) default keymap sends.
pub const CONSUMER_TABLE: &[(&str, u16, u16, i16, u16, u16)] = &[
    ("brightnessup", 0x06F, 225, 2, NO, 0),
    ("brightnessdown", 0x070, 224, 3, NO, 0),
    ("nextsong", 0x0B5, 163, 19, NO, 0xB0),
    ("prevsong", 0x0B6, 165, 20, NO, 0xB1),
    ("playpause", 0x0CD, 164, 16, NO, 0xB3),
    ("mute", 0x0E2, 113, 7, NO, 0xAD),
    ("volumeup", 0x0E9, 115, 0, NO, 0xAF),
    ("volumedown", 0x0EA, 114, 1, NO, 0xAE),
    // Mission Control / Launchpad: Linux maps 0x29F to KEY_SCALE; the macOS keycodes
    // (0xA0 / 0x83) are the ones Apple keyboards use. Windows has no equivalent.
    ("missioncontrol", 0x29F, 120, NX_NONE, 0xA0, 0),
    ("launchpad", 0x2A0, NO, NX_NONE, 0x83, 0),
    ("stop", 0x0B7, 166, NX_NONE, NO, 0xB2),
    ("eject", 0x0B8, 161, 14, NO, 0),
];

#[inline]
pub fn is_consumer(id: u16) -> bool {
    id & 0xF000 == CONSUMER
}
fn consumer_row(id: u16) -> Option<&'static (&'static str, u16, u16, i16, u16, u16)> {
    if !is_consumer(id) {
        return None;
    }
    CONSUMER_TABLE.iter().find(|r| r.1 == id & 0x0FFF)
}
fn consumer_index(id: u16) -> Option<usize> {
    if !is_consumer(id) {
        return None;
    }
    CONSUMER_TABLE.iter().position(|r| r.1 == id & 0x0FFF)
}
/// macOS NX_KEYTYPE (from a system-defined event) -> id. 17/18 are the older next/previous codes.
pub fn from_mac_nx(nx: i16) -> Option<u16> {
    let nx = match nx {
        17 => 19,
        18 => 20,
        n => n,
    };
    CONSUMER_TABLE.iter().find(|r| r.3 == nx && nx >= 0).map(|r| CONSUMER | r.1)
}
pub fn to_mac_nx(id: u16) -> Option<i16> {
    consumer_row(id).map(|r| r.3).filter(|&n| n >= 0)
}
pub fn from_win_vk(vk: u16) -> Option<u16> {
    CONSUMER_TABLE.iter().find(|r| r.5 == vk && vk != 0).map(|r| CONSUMER | r.1)
}
pub fn to_win_vk(id: u16) -> Option<u16> {
    consumer_row(id).map(|r| r.5).filter(|&v| v != 0)
}

pub fn name(hid: u16) -> &'static str {
    if let Some(r) = consumer_row(hid) {
        return r.0;
    }
    hid_row(hid).map(|r| r.0).unwrap_or("?")
}
pub fn from_name(n: &str) -> Option<u16> {
    let n = n.to_ascii_lowercase();
    let n = match n.as_str() {
        "return" => "enter",
        "escape" => "esc",
        "del" => "delete",
        "ins" => "insert",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        other => other,
    };
    TABLE
        .iter()
        .find(|r| r.0 == n)
        .map(|r| r.1)
        .or_else(|| CONSUMER_TABLE.iter().find(|r| r.0 == n).map(|r| CONSUMER | r.1))
}

/// Key that types `c` on a US ANSI layout (shifted characters map to their key).
pub fn from_char(c: char) -> Option<u16> {
    let c = c.to_ascii_lowercase();
    let name = match c {
        'a'..='z' | '0'..='9' => return from_name(c.encode_utf8(&mut [0; 4])),
        '!' => "1",
        '@' => "2",
        '#' => "3",
        '$' => "4",
        '%' => "5",
        '^' => "6",
        '&' => "7",
        '*' => "8",
        '(' => "9",
        ')' => "0",
        '-' | '_' => "minus",
        '=' | '+' => "equal",
        '[' | '{' => "leftbracket",
        ']' | '}' => "rightbracket",
        '\\' | '|' => "backslash",
        ';' | ':' => "semicolon",
        '\'' | '"' => "quote",
        '`' | '~' => "grave",
        ',' | '<' => "comma",
        '.' | '>' => "period",
        '/' | '?' => "slash",
        ' ' => "space",
        _ => return None,
    };
    from_name(name)
}

#[inline]
pub fn from_mac(code: u16) -> Option<u16> {
    BY_MAC
        .get(code as usize)
        .and_then(|&s| row(s))
        .map(|r| r.1)
        .or_else(|| CONSUMER_TABLE.iter().find(|r| r.4 == code && code != NO).map(|r| CONSUMER | r.1))
}
#[inline]
pub fn to_mac(hid: u16) -> Option<u16> {
    if is_consumer(hid) {
        return consumer_row(hid).map(|r| r.4).filter(|&c| c != NO);
    }
    hid_row(hid).map(|r| r.2).filter(|&c| c != NO)
}
#[inline]
pub fn from_evdev(code: u16) -> Option<u16> {
    BY_EVDEV
        .get(code as usize)
        .and_then(|&s| row(s))
        .map(|r| r.1)
        .or_else(|| CONSUMER_TABLE.iter().find(|r| r.2 == code && code != NO).map(|r| CONSUMER | r.1))
}
#[inline]
pub fn to_evdev(hid: u16) -> Option<u16> {
    if is_consumer(hid) {
        return consumer_row(hid).map(|r| r.2).filter(|&c| c != NO);
    }
    hid_row(hid).map(|r| r.3).filter(|&c| c != NO)
}
/// `scan` is the set-1 scancode; `extended` = the E0 prefix.
#[inline]
pub fn from_win(scan: u16, extended: bool) -> Option<u16> {
    let idx = if extended { (scan & 0xFF) + 0x100 } else { scan & 0xFF };
    BY_WIN.get(idx as usize).and_then(|&s| row(s)).map(|r| r.1)
}
/// Returns (scancode, extended).
#[inline]
pub fn to_win(hid: u16) -> Option<(u16, bool)> {
    if is_consumer(hid) {
        return None;
    }
    hid_row(hid).map(|r| r.4).filter(|&c| c != NO).map(|c| (c & 0xFF, c >= 0xE000))
}

/// Modifier bit for a HID modifier usage (0xE0..=0xE7), or 0.
#[inline]
pub fn mod_bit(hid: u16) -> u8 {
    match hid {
        0xE0 | 0xE4 => MOD_CTRL,
        0xE1 | 0xE5 => MOD_SHIFT,
        0xE2 | 0xE6 => MOD_ALT,
        0xE3 | 0xE7 => MOD_META,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub mods: u8,
    pub hid: u16,
}

pub fn parse_hotkey(s: &str) -> Result<Hotkey, String> {
    let mut mods = 0u8;
    let mut key = None;
    for part in s.split('+').map(|p| p.trim().to_ascii_lowercase()) {
        let m = match part.as_str() {
            "ctrl" | "control" => MOD_CTRL,
            "shift" => MOD_SHIFT,
            "alt" | "option" | "opt" => MOD_ALT,
            "meta" | "cmd" | "command" | "super" | "win" => MOD_META,
            _ => 0,
        };
        if m != 0 {
            mods |= m;
        } else if key.is_some() {
            return Err(format!("hotkey '{s}' has more than one non-modifier key"));
        } else {
            key = Some(from_name(&part).ok_or_else(|| format!("unknown key '{part}'"))?);
        }
    }
    let hid = key.ok_or_else(|| format!("hotkey '{s}' has no key"))?;
    Ok(Hotkey { mods, hid })
}

pub fn hotkey_name(h: &Hotkey) -> String {
    let mut s = String::new();
    for (bit, n) in [(MOD_CTRL, "ctrl+"), (MOD_SHIFT, "shift+"), (MOD_ALT, "alt+"), (MOD_META, "meta+")] {
        if h.mods & bit != 0 {
            s.push_str(n);
        }
    }
    s.push_str(name(h.hid));
    s
}

/// Fixed-size set of key ids (keyboard usages 0..=255 plus consumer keys), no allocation.
#[derive(Clone, Copy, Default)]
pub struct KeySet([u64; 5]);

#[inline]
fn slot(k: u16) -> Option<usize> {
    if k < 0x100 {
        Some(k as usize)
    } else {
        consumer_index(k).map(|i| 0x100 + i)
    }
}

impl KeySet {
    #[inline]
    pub fn insert(&mut self, k: u16) -> bool {
        let Some(i) = slot(k) else { return false };
        let had = self.0[i >> 6] & (1 << (i & 63)) != 0;
        self.0[i >> 6] |= 1 << (i & 63);
        !had
    }
    #[inline]
    pub fn remove(&mut self, k: u16) -> bool {
        let Some(i) = slot(k) else { return false };
        let had = self.0[i >> 6] & (1 << (i & 63)) != 0;
        self.0[i >> 6] &= !(1 << (i & 63));
        had
    }
    #[inline]
    pub fn contains(&self, k: u16) -> bool {
        slot(k).is_some_and(|i| self.0[i >> 6] & (1 << (i & 63)) != 0)
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == [0; 5]
    }
    pub fn clear(&mut self) {
        self.0 = [0; 5]
    }
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0u16..256)
            .chain(CONSUMER_TABLE.iter().map(|r| CONSUMER | r.1))
            .filter(move |&k| self.contains(k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chars_map_to_keys() {
        assert_eq!(from_char('a'), Some(0x04));
        assert_eq!(from_char('D'), Some(0x07));
        assert_eq!(from_char('0'), Some(0x27));
        assert_eq!(from_char('_'), Some(0x2D));
        assert_eq!(from_char('/'), Some(0x38));
        assert_eq!(from_char('é'), None);
    }

    #[test]
    fn table_is_consistent() {
        for r in TABLE {
            assert_eq!(hid_row(r.1).unwrap().0, r.0, "hid dup {}", r.0);
            if r.2 != NO {
                assert_eq!(from_mac(r.2), Some(r.1), "mac {}", r.0);
            }
            assert_eq!(from_evdev(r.3), Some(r.1), "evdev {}", r.0);
            let (s, e) = to_win(r.1).unwrap();
            assert_eq!(from_win(s, e), Some(r.1), "win {}", r.0);
            assert_eq!(from_name(r.0), Some(r.1));
        }
        assert!(TABLE.len() < 255);
        assert!(CONSUMER_TABLE.len() <= 64);
        for r in CONSUMER_TABLE {
            let id = CONSUMER | r.1;
            assert_eq!(from_name(r.0), Some(id));
            assert_eq!(name(id), r.0);
            if r.2 != NO {
                assert_eq!(from_evdev(r.2), Some(id), "evdev {}", r.0);
                assert_eq!(to_evdev(id), Some(r.2));
            }
            if r.3 >= 0 {
                assert_eq!(from_mac_nx(r.3), Some(id), "nx {}", r.0);
            }
            if r.4 != NO {
                assert_eq!(from_mac(r.4), Some(id), "mac {}", r.0);
            }
            if r.5 != 0 {
                assert_eq!(from_win_vk(r.5), Some(id));
            }
        }
        assert_eq!(from_mac_nx(17), Some(CONSUMER | 0xB5)); // legacy NX next
    }

    #[test]
    fn hotkeys() {
        let h = parse_hotkey("Ctrl+Alt+F19").unwrap();
        assert_eq!(h, Hotkey { mods: MOD_CTRL | MOD_ALT, hid: 0x6E });
        assert_eq!(hotkey_name(&h), "ctrl+alt+f19");
        assert!(parse_hotkey("ctrl+a+b").is_err());
        assert!(parse_hotkey("ctrl").is_err());
    }

    #[test]
    fn keyset() {
        let mut s = KeySet::default();
        assert!(s.insert(0xE7));
        assert!(!s.insert(0xE7));
        assert!(s.contains(0xE7) && !s.contains(0x04));
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![0xE7]);
        assert!(s.remove(0xE7) && s.is_empty());
        let vol = from_name("volumeup").unwrap();
        assert!(s.insert(vol) && s.contains(vol) && !s.contains(0xE9));
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![vol]);
        assert!(!s.insert(CONSUMER | 0x123)); // unknown consumer usage is not tracked
    }
}
