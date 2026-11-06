//! Windows backend: low-level keyboard/mouse hooks (capture + swallow) and
//! SendInput (inject). No admin rights needed; input cannot reach elevated
//! windows unless keyhop itself runs elevated (UIPI).

use super::MAGIC;
use crate::config::{Config, Edge};
use crate::engine::{at_edge, Handler, Input, Verdict};
use crate::keys;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::{Arc, OnceLock};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, KEYBDINPUT, MOUSEINPUT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, GetMessageW, GetSystemMetrics, SetCursorPos, SetWindowsHookExW, KBDLLHOOKSTRUCT,
    MSG, MSLLHOOKSTRUCT,
};

const WH_KEYBOARD_LL: i32 = 13;
const WH_MOUSE_LL: i32 = 14;
const LLKHF_EXTENDED: u32 = 0x01;
const WM_KEYDOWN: usize = 0x100;
const WM_KEYUP: usize = 0x101;
const WM_SYSKEYDOWN: usize = 0x104;
const WM_SYSKEYUP: usize = 0x105;
const WM_MOUSEMOVE: usize = 0x200;
const WM_LBUTTONDOWN: usize = 0x201;
const WM_LBUTTONUP: usize = 0x202;
const WM_RBUTTONDOWN: usize = 0x204;
const WM_RBUTTONUP: usize = 0x205;
const WM_MBUTTONDOWN: usize = 0x207;
const WM_MBUTTONUP: usize = 0x208;
const WM_MOUSEWHEEL: usize = 0x20A;
const WM_XBUTTONDOWN: usize = 0x20B;
const WM_XBUTTONUP: usize = 0x20C;
const WM_MOUSEHWHEEL: usize = 0x20E;

const INPUT_MOUSE: u32 = 0;
const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_EXTENDEDKEY: u32 = 0x1;
const KEYEVENTF_KEYUP: u32 = 0x2;
const KEYEVENTF_SCANCODE: u32 = 0x8;
const MOUSEEVENTF_MOVE: u32 = 0x1;
const MOUSEEVENTF_WHEEL: u32 = 0x800;
const MOUSEEVENTF_HWHEEL: u32 = 0x1000;

const SM_XVIRTUALSCREEN: i32 = 76;
const SM_YVIRTUALSCREEN: i32 = 77;
const SM_CXVIRTUALSCREEN: i32 = 78;
const SM_CYVIRTUALSCREEN: i32 = 79;

static HANDLER: OnceLock<Arc<dyn Handler>> = OnceLock::new();

fn cursor() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

unsafe extern "system" fn kbd_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let k = &*(l as *const KBDLLHOOKSTRUCT);
        if k.dwExtraInfo != MAGIC as usize {
            let id = keys::from_win_vk(k.vkCode as u16)
                .or_else(|| keys::from_win(k.scanCode as u16, k.flags & LLKHF_EXTENDED != 0));
            if let (Some(h), Some(hid)) = (HANDLER.get(), id) {
                let down = matches!(w, WM_KEYDOWN | WM_SYSKEYDOWN);
                if matches!(w, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP)
                    && h.on_input(&Input::Key { hid, down, repeat: false }) == Verdict::Swallow
                {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, w, l)
}

unsafe extern "system" fn mouse_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let m = &*(l as *const MSLLHOOKSTRUCT);
        if m.dwExtraInfo != MAGIC as usize {
            if let Some(h) = HANDLER.get() {
                let hi = (m.mouseData >> 16) as u16;
                let input = match w {
                    WM_MOUSEMOVE => {
                        // Not applied yet: GetCursorPos is still the previous position.
                        let (cx, cy) = cursor();
                        Some(Input::Motion { dx: m.pt.x - cx, dy: m.pt.y - cy, pos: Some((m.pt.x as f64, m.pt.y as f64)) })
                    }
                    WM_LBUTTONDOWN | WM_LBUTTONUP => Some(Input::Button { button: 1, down: w == WM_LBUTTONDOWN }),
                    WM_RBUTTONDOWN | WM_RBUTTONUP => Some(Input::Button { button: 2, down: w == WM_RBUTTONDOWN }),
                    WM_MBUTTONDOWN | WM_MBUTTONUP => Some(Input::Button { button: 3, down: w == WM_MBUTTONDOWN }),
                    WM_XBUTTONDOWN | WM_XBUTTONUP => {
                        Some(Input::Button { button: if hi == 1 { 4 } else { 5 }, down: w == WM_XBUTTONDOWN })
                    }
                    WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                        let d = hi as i16 as i32;
                        let continuous = d % 120 != 0; // precision touchpads / free-spin hi-res wheels
                        Some(if w == WM_MOUSEWHEEL {
                            Input::Wheel { dx: 0, dy: d, continuous }
                        } else {
                            Input::Wheel { dx: d, dy: 0, continuous }
                        })
                    }
                    _ => None,
                };
                if let Some(i) = input {
                    if h.on_input(&i) == Verdict::Swallow {
                        return 1;
                    }
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, w, l)
}

pub fn run_capture(h: Arc<dyn Handler>, _grab_mice: bool) -> Result<(), String> {
    let _ = HANDLER.set(h);
    unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        let kh = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_proc), module, 0);
        let mh = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
        if kh.is_null() || mh.is_null() {
            return Err("SetWindowsHookEx failed".into());
        }
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
    }
    Ok(())
}

pub fn screen_bounds() -> Option<(f64, f64, f64, f64)> {
    unsafe {
        let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
        let (w, h) = (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
        (w > 0 && h > 0).then(|| (x as f64, y as f64, (x + w) as f64, (y + h) as f64))
    }
}

/// Blocking a WM_MOUSEMOVE in the hook already pins the cursor; nothing to do.
pub fn set_remote(_on: bool) {}

pub fn smooth_scale(_cfg: &Config) -> f32 {
    120.0
}

/// Detach from the console when started at login (`run --background`).
pub fn detach_console() {
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
    }
}

fn mouse(dx: i32, dy: i32, data: u32, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: data, dwFlags: flags, time: 0, dwExtraInfo: MAGIC as usize },
        },
    }
}

pub struct Injector;

impl Injector {
    pub fn new(_cfg: &Config) -> Result<Injector, String> {
        Ok(Injector)
    }

    pub fn key(&mut self, hid: u16, state: u8) {
        if let Some(vk) = keys::to_win_vk(hid) {
            // Media keys: virtual-key codes, extended.
            let flags = KEYEVENTF_EXTENDEDKEY | if state == 0 { KEYEVENTF_KEYUP } else { 0 };
            send(&[INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: MAGIC as usize },
                },
            }]);
            return;
        }
        let Some((scan, ext)) = keys::to_win(hid) else { return };
        let mut flags = KEYEVENTF_SCANCODE;
        if ext {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if state == 0 {
            flags |= KEYEVENTF_KEYUP;
        }
        send(&[INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: 0, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: MAGIC as usize },
            },
        }]);
    }

    pub fn motion(&mut self, dx: i32, dy: i32) {
        send(&[mouse(dx, dy, 0, MOUSEEVENTF_MOVE)]);
    }

    pub fn button(&mut self, b: u8, down: bool) {
        let (flags, data) = match (b, down) {
            (1, true) => (0x02, 0),
            (1, false) => (0x04, 0),
            (2, true) => (0x08, 0),
            (2, false) => (0x10, 0),
            (3, true) => (0x20, 0),
            (3, false) => (0x40, 0),
            (4, true) => (0x80, 1),
            (4, false) => (0x100, 1),
            (5, true) => (0x80, 2),
            (5, false) => (0x100, 2),
            _ => return,
        };
        send(&[mouse(0, 0, data, flags)]);
    }

    /// Windows wheel units are already 120 per notch; sub-notch deltas scroll smoothly in modern apps.
    pub fn wheel(&mut self, dx: i32, dy: i32) {
        let mut v = Vec::with_capacity(2);
        if dy != 0 {
            v.push(mouse(0, 0, dy as u32, MOUSEEVENTF_WHEEL));
        }
        if dx != 0 {
            v.push(mouse(0, 0, dx as u32, MOUSEEVENTF_HWHEEL));
        }
        if !v.is_empty() {
            send(&v);
        }
    }

    pub fn smooth_scroll(&mut self, dx: i32, dy: i32) {
        self.wheel(dx, dy);
    }

    pub fn warp(&mut self, edge: Edge, ratio: f32) {
        let Some((x0, y0, x1, y1)) = screen_bounds() else { return };
        let r = ratio as f64;
        let (x, y) = match edge {
            Edge::Left => (x0 + 2.0, y0 + r * (y1 - y0)),
            Edge::Right => (x1 - 3.0, y0 + r * (y1 - y0)),
            Edge::Top => (x0 + r * (x1 - x0), y0 + 2.0),
            Edge::Bottom => (x0 + r * (x1 - x0), y1 - 3.0),
        };
        unsafe {
            SetCursorPos(x as i32, y as i32);
        }
    }

    pub fn edge_ratio(&mut self, edge: Edge) -> Option<f32> {
        let (x, y) = cursor();
        at_edge(edge, screen_bounds()?, x as f64, y as f64)
    }
}
