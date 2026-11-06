//! Compact binary messages carried inside Noise transport frames.

pub const KIND_SESSION: u8 = 1;
pub const KIND_FILE: u8 = 2;
pub const KIND_PAIR: u8 = 3;

/// Largest plaintext payload per frame (Noise max 65535 incl. 16-byte tag).
pub const MAX_PLAIN: usize = 65535 - 16;
pub const FILE_CHUNK: usize = MAX_PLAIN - 1;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Hello { kind: u8, name: String },
    /// Controller's cursor left through `edge` at `ratio` (0..1) along that edge.
    Enter { edge: u8, ratio: f32 },
    /// Controlled side hands control back; `ratio` along the return edge.
    Leave { ratio: f32 },
    /// state: 0 up, 1 down, 2 repeat
    Key { hid: u16, state: u8 },
    Motion { dx: i16, dy: i16 },
    /// 1 left, 2 right, 3 middle, 4 back, 5 forward
    Button { button: u8, down: bool },
    /// 120 units per wheel notch; dy > 0 = up, dx > 0 = right
    Wheel { dx: i16, dy: i16 },
    Ping,
    FileStart { size: u64, name: String },
    FileData(Vec<u8>),
    FileEnd,
    FileAck { ok: bool },
    Confirm { yes: bool, name: String },
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    let b = &s.as_bytes()[..s.len().min(1024)];
    out.extend_from_slice(&(b.len() as u16).to_le_bytes());
    out.extend_from_slice(b);
}

impl Msg {
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.clear();
        match self {
            Msg::Hello { kind, name } => {
                out.extend_from_slice(&[0x00, *kind]);
                put_str(out, name);
            }
            Msg::Enter { edge, ratio } => {
                out.extend_from_slice(&[0x01, *edge]);
                out.extend_from_slice(&ratio.to_le_bytes());
            }
            Msg::Leave { ratio } => {
                out.push(0x02);
                out.extend_from_slice(&ratio.to_le_bytes());
            }
            Msg::Key { hid, state } => {
                out.push(0x03);
                out.extend_from_slice(&hid.to_le_bytes());
                out.push(*state);
            }
            Msg::Motion { dx, dy } => {
                out.push(0x04);
                out.extend_from_slice(&dx.to_le_bytes());
                out.extend_from_slice(&dy.to_le_bytes());
            }
            Msg::Button { button, down } => out.extend_from_slice(&[0x05, *button, *down as u8]),
            Msg::Wheel { dx, dy } => {
                out.push(0x06);
                out.extend_from_slice(&dx.to_le_bytes());
                out.extend_from_slice(&dy.to_le_bytes());
            }
            Msg::Ping => out.push(0x07),
            Msg::FileStart { size, name } => {
                out.push(0x10);
                out.extend_from_slice(&size.to_le_bytes());
                put_str(out, name);
            }
            Msg::FileData(d) => {
                out.push(0x11);
                out.extend_from_slice(d);
            }
            Msg::FileEnd => out.push(0x12),
            Msg::FileAck { ok } => out.extend_from_slice(&[0x13, *ok as u8]),
            Msg::Confirm { yes, name } => {
                out.extend_from_slice(&[0x20, *yes as u8]);
                put_str(out, name);
            }
        }
    }

    pub fn decode(b: &[u8]) -> Option<Msg> {
        let (&tag, p) = b.split_first()?;
        let u16_at = |i: usize| p.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
        let i16_at = |i: usize| u16_at(i).map(|v| v as i16);
        let f32_at = |i: usize| p.get(i..i + 4).map(|s| f32::from_le_bytes(s.try_into().unwrap()));
        let str_at = |i: usize| -> Option<String> {
            let n = u16_at(i)? as usize;
            String::from_utf8(p.get(i + 2..i + 2 + n)?.to_vec()).ok()
        };
        Some(match tag {
            0x00 => Msg::Hello { kind: *p.first()?, name: str_at(1)? },
            0x01 => Msg::Enter { edge: *p.first()?, ratio: f32_at(1)? },
            0x02 => Msg::Leave { ratio: f32_at(0)? },
            0x03 => Msg::Key { hid: u16_at(0)?, state: *p.get(2)? },
            0x04 => Msg::Motion { dx: i16_at(0)?, dy: i16_at(2)? },
            0x05 => Msg::Button { button: *p.first()?, down: *p.get(1)? != 0 },
            0x06 => Msg::Wheel { dx: i16_at(0)?, dy: i16_at(2)? },
            0x07 => Msg::Ping,
            0x10 => Msg::FileStart {
                size: u64::from_le_bytes(p.get(0..8)?.try_into().ok()?),
                name: str_at(8)?,
            },
            0x11 => Msg::FileData(p.to_vec()),
            0x12 => Msg::FileEnd,
            0x13 => Msg::FileAck { ok: *p.first()? != 0 },
            0x20 => Msg::Confirm { yes: *p.first()? != 0, name: str_at(1)? },
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let msgs = [
            Msg::Hello { kind: KIND_SESSION, name: "mac".into() },
            Msg::Enter { edge: 1, ratio: 0.25 },
            Msg::Leave { ratio: 0.75 },
            Msg::Key { hid: 0xE3, state: 2 },
            Msg::Motion { dx: -300, dy: 7 },
            Msg::Button { button: 4, down: true },
            Msg::Wheel { dx: 0, dy: -120 },
            Msg::Ping,
            Msg::FileStart { size: 1 << 40, name: "ä b.txt".into() },
            Msg::FileData(vec![1, 2, 3]),
            Msg::FileEnd,
            Msg::FileAck { ok: true },
            Msg::Confirm { yes: false, name: "pc".into() },
        ];
        let mut buf = Vec::new();
        for m in msgs {
            m.encode(&mut buf);
            assert_eq!(Msg::decode(&buf), Some(m));
        }
        assert_eq!(Msg::decode(&[0x03, 1]), None);
        assert_eq!(Msg::decode(&[0xFF]), None);
    }
}
