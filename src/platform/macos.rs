//! macOS backend: one CGEventTap (capture + swallow) and CGEventPost (inject).
//! Raw FFI, no Objective-C runtime, no extra crates.
//! Needs Accessibility + Input Monitoring for the keyhop binary.

#![allow(non_upper_case_globals)]

use super::MAGIC;
use crate::config::{Config, Edge};
use crate::engine::{at_edge, Handler, Input, Verdict};
use crate::keys;
use std::ffi::{c_char, c_void};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

type Ref = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

type TapCallback = unsafe extern "C" fn(proxy: Ref, ty: u32, event: Ref, info: *mut c_void) -> Ref;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: u64, cb: TapCallback, info: *mut c_void) -> Ref;
    fn CGEventTapEnable(tap: Ref, enable: bool);
    fn CGEventGetIntegerValueField(e: Ref, field: u32) -> i64;
    fn CGEventSetIntegerValueField(e: Ref, field: u32, v: i64);
    fn CGEventGetDoubleValueField(e: Ref, field: u32) -> f64;
    fn CGEventGetLocation(e: Ref) -> CGPoint;
    fn CGEventSetFlags(e: Ref, flags: u64);
    fn CGEventSetType(e: Ref, ty: u32);
    fn CGEventCreate(src: Ref) -> Ref;
    fn CGEventCreateKeyboardEvent(src: Ref, key: u16, down: bool) -> Ref;
    fn CGEventCreateMouseEvent(src: Ref, ty: u32, pos: CGPoint, button: u32) -> Ref;
    fn CGEventCreateScrollWheelEvent2(src: Ref, units: u32, count: u32, w1: i32, w2: i32, w3: i32) -> Ref;
    fn CGEventPost(tap: u32, e: Ref);
    fn CGEventSourceCreate(state: i32) -> Ref;
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    fn CGAssociateMouseAndMouseCursorPosition(connected: u32) -> i32;
    fn CGSetLocalEventsSuppressionInterval(seconds: f64) -> i32;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> u8;
    static kAXTrustedCheckOptionPrompt: Ref;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDRequestAccess(request: u32) -> bool;
}
const HID_REQUEST_LISTEN_EVENT: u32 = 1; // kIOHIDRequestTypeListenEvent

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(alloc: Ref, port: Ref, order: isize) -> Ref;
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(rl: Ref, src: Ref, mode: Ref);
    fn CFRunLoopRun();
    fn CFRelease(r: Ref);
    fn CFDictionaryCreate(alloc: Ref, keys: *const Ref, values: *const Ref, n: isize, kcb: Ref, vcb: Ref) -> Ref;
    static kCFRunLoopCommonModes: Ref;
    static kCFBooleanTrue: Ref;
}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Ref;
    fn sel_registerName(name: *const c_char) -> Ref;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}
// NSEvent lives in AppKit.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

// Event types
const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const MOUSE_MOVED: u32 = 5;
const LEFT_DRAGGED: u32 = 6;
const RIGHT_DRAGGED: u32 = 7;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const SCROLL: u32 = 22;
const SYSTEM_DEFINED: u32 = 14; // NX_SYSDEFINED: media / brightness keys
const AUX_CONTROL_BUTTONS: i16 = 8;
const OTHER_DOWN: u32 = 25;
const OTHER_UP: u32 = 26;
const OTHER_DRAGGED: u32 = 27;
const TAP_DISABLED_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_USER: u32 = 0xFFFF_FFFF;

// Event fields
const F_CLICK_STATE: u32 = 1;
const F_BUTTON_NUMBER: u32 = 3;
const F_DELTA_X: u32 = 4;
const F_DELTA_Y: u32 = 5;
const F_AUTOREPEAT: u32 = 8;
const F_KEYCODE: u32 = 9;
const F_SCROLL_FIXED1: u32 = 93;
const F_SCROLL_FIXED2: u32 = 94;
const F_SCROLL_POINT1: u32 = 96;
const F_SCROLL_POINT2: u32 = 97;
const F_SCROLL_CONTINUOUS: u32 = 88;
const F_USER_DATA: u32 = 42;

const HID_TAP: u32 = 0;
const SCROLL_PIXEL: u32 = 0;

// Modifier flags (device-independent | device-dependent left/right bits)
const NX_NONCOALESCED: u64 = 0x100;
fn mac_mod_flags(code: u16) -> u64 {
    match code {
        0x38 => 0x20000 | 0x02,   // left shift
        0x3C => 0x20000 | 0x04,   // right shift
        0x3B => 0x40000 | 0x01,   // left control
        0x3E => 0x40000 | 0x2000, // right control
        0x3A => 0x80000 | 0x20,   // left option
        0x3D => 0x80000 | 0x40,   // right option
        0x37 => 0x100000 | 0x08,  // left command
        0x36 => 0x100000 | 0x10,  // right command
        0x39 => 0x10000,          // caps lock
        _ => 0,
    }
}

static TAP: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static HANDLER: OnceLock<Arc<dyn Handler>> = OnceLock::new();
/// Bitmask of physically held modifier keycodes 0x36..=0x3E (flagsChanged has no up/down).
static MODS_HELD: AtomicU64 = AtomicU64::new(0);

#[inline]
unsafe fn field(e: Ref, f: u32) -> i64 {
    CGEventGetIntegerValueField(e, f)
}

fn mac_button(n: i64) -> u8 {
    match n {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 5,
        n => (n.clamp(0, 7) + 1) as u8,
    }
}

// ---- media keys: NX_SYSDEFINED events, decoded/created through NSEvent ----

unsafe fn sel(name: &[u8]) -> Ref {
    sel_registerName(name.as_ptr() as *const c_char)
}

unsafe fn nsevent() -> Ref {
    objc_getClass(c"NSEvent".as_ptr())
}

/// (NX_KEYTYPE, down, repeat) for an aux-control-button system-defined event.
unsafe fn read_aux(e: Ref) -> Option<(i16, bool, bool)> {
    type FromCg = unsafe extern "C" fn(Ref, Ref, Ref) -> Ref;
    type Subtype = unsafe extern "C" fn(Ref, Ref) -> i16;
    type Data1 = unsafe extern "C" fn(Ref, Ref) -> isize;
    let send = objc_msgSend as unsafe extern "C" fn();
    let pool = objc_autoreleasePoolPush();
    let ns = std::mem::transmute::<unsafe extern "C" fn(), FromCg>(send)(nsevent(), sel(b"eventWithCGEvent:\0"), e);
    let mut out = None;
    if !ns.is_null() && std::mem::transmute::<unsafe extern "C" fn(), Subtype>(send)(ns, sel(b"subtype\0")) == AUX_CONTROL_BUTTONS {
        let d1 = std::mem::transmute::<unsafe extern "C" fn(), Data1>(send)(ns, sel(b"data1\0")) as u64;
        let key = ((d1 >> 16) & 0xFFFF) as i16;
        let state = (d1 >> 8) & 0xFF;
        out = Some((key, state == 0x0A, d1 & 1 != 0));
    }
    objc_autoreleasePoolPop(pool);
    out
}

unsafe fn post_aux(nx: i16, down: bool) {
    type Other = unsafe extern "C" fn(Ref, Ref, u64, CGPoint, u64, f64, isize, Ref, i16, isize, isize) -> Ref;
    type GetCg = unsafe extern "C" fn(Ref, Ref) -> Ref;
    let send = objc_msgSend as unsafe extern "C" fn();
    let pool = objc_autoreleasePoolPush();
    let state: u64 = if down { 0x0A } else { 0x0B };
    let ns = std::mem::transmute::<unsafe extern "C" fn(), Other>(send)(
        nsevent(),
        sel(b"otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:\0"),
        SYSTEM_DEFINED as u64,
        CGPoint::default(),
        state << 8,
        0.0,
        0,
        null_mut(),
        AUX_CONTROL_BUTTONS,
        (((nx as u64) << 16) | (state << 8)) as isize,
        -1,
    );
    if !ns.is_null() {
        let cg = std::mem::transmute::<unsafe extern "C" fn(), GetCg>(send)(ns, sel(b"CGEvent\0"));
        if !cg.is_null() {
            // Owned by the NSEvent: mark and post, but don't release.
            CGEventSetIntegerValueField(cg, F_USER_DATA, MAGIC);
            CGEventPost(HID_TAP, cg);
        }
    }
    objc_autoreleasePoolPop(pool);
}

unsafe extern "C" fn tap_cb(_proxy: Ref, ty: u32, e: Ref, _info: *mut c_void) -> Ref {
    if ty == TAP_DISABLED_TIMEOUT || ty == TAP_DISABLED_USER {
        CGEventTapEnable(TAP.load(Relaxed), true);
        return e;
    }
    if field(e, F_USER_DATA) == MAGIC {
        return e;
    }
    let Some(h) = HANDLER.get() else { return e };
    let verdict = |inp: Input| h.on_input(&inp);
    let v = match ty {
        KEY_DOWN | KEY_UP => match keys::from_mac(field(e, F_KEYCODE) as u16) {
            Some(hid) => verdict(Input::Key { hid, down: ty == KEY_DOWN, repeat: field(e, F_AUTOREPEAT) != 0 }),
            None => Verdict::Pass,
        },
        FLAGS_CHANGED => {
            let code = field(e, F_KEYCODE) as u16;
            match keys::from_mac(code) {
                Some(0x39) => {
                    // Caps Lock reports one event per press: treat as a tap.
                    let a = verdict(Input::Key { hid: 0x39, down: true, repeat: false });
                    let b = verdict(Input::Key { hid: 0x39, down: false, repeat: false });
                    if a == Verdict::Swallow || b == Verdict::Swallow { Verdict::Swallow } else { Verdict::Pass }
                }
                Some(hid) if (0x36..=0x3E).contains(&code) => {
                    let bit = 1u64 << (code - 0x36);
                    let was = MODS_HELD.fetch_xor(bit, Relaxed) & bit != 0;
                    verdict(Input::Key { hid, down: !was, repeat: false })
                }
                _ => Verdict::Pass,
            }
        }
        MOUSE_MOVED | LEFT_DRAGGED | RIGHT_DRAGGED | OTHER_DRAGGED => {
            let p = CGEventGetLocation(e);
            verdict(Input::Motion {
                dx: field(e, F_DELTA_X) as i32,
                dy: field(e, F_DELTA_Y) as i32,
                pos: Some((p.x, p.y)),
            })
        }
        LEFT_DOWN | LEFT_UP | RIGHT_DOWN | RIGHT_UP | OTHER_DOWN | OTHER_UP => {
            let down = matches!(ty, LEFT_DOWN | RIGHT_DOWN | OTHER_DOWN);
            verdict(Input::Button { button: mac_button(field(e, F_BUTTON_NUMBER)), down })
        }
        SCROLL => {
            let continuous = field(e, F_SCROLL_CONTINUOUS) != 0;
            let (dy, dx) = if continuous {
                // pixels -> approx. 1/120 notch units (~12 px per notch)
                (field(e, F_SCROLL_POINT1) as i32 * 10, -(field(e, F_SCROLL_POINT2) as i32) * 10)
            } else {
                (
                    (CGEventGetDoubleValueField(e, F_SCROLL_FIXED1) * 120.0).round() as i32,
                    -(CGEventGetDoubleValueField(e, F_SCROLL_FIXED2) * 120.0).round() as i32,
                )
            };
            if dx == 0 && dy == 0 {
                Verdict::Pass
            } else {
                verdict(Input::Wheel { dx, dy, continuous })
            }
        }
        SYSTEM_DEFINED => match read_aux(e).and_then(|(nx, down, rep)| Some((keys::from_mac_nx(nx)?, down, rep))) {
            Some((hid, down, repeat)) => verdict(Input::Key { hid, down, repeat }),
            None => Verdict::Pass,
        },
        _ => Verdict::Pass,
    };
    if v == Verdict::Swallow {
        null_mut()
    } else {
        e
    }
}

pub fn run_capture(h: Arc<dyn Handler>, _grab_mice: bool) -> Result<(), String> {
    let _ = HANDLER.set(h);
    let mask: u64 = [
        LEFT_DOWN, LEFT_UP, RIGHT_DOWN, RIGHT_UP, MOUSE_MOVED, LEFT_DRAGGED, RIGHT_DRAGGED, KEY_DOWN, KEY_UP,
        FLAGS_CHANGED, SCROLL, OTHER_DOWN, OTHER_UP, OTHER_DRAGGED, SYSTEM_DEFINED,
    ]
    .iter()
    .fold(0, |m, t| m | (1u64 << t));
    unsafe {
        CGSetLocalEventsSuppressionInterval(0.0);
        let tap = CGEventTapCreate(HID_TAP, 0, 0, mask, tap_cb, null_mut());
        if tap.is_null() {
            // Ask macOS to show its own permission prompts for this binary (it
            // only does so once per binary). Under the login service, launchd
            // restarts keyhop, which works as soon as both are switched on.
            let (k, v) = ([kAXTrustedCheckOptionPrompt], [kCFBooleanTrue]);
            let opts = CFDictionaryCreate(null_mut(), k.as_ptr(), v.as_ptr(), 1, null_mut(), null_mut());
            let trusted = AXIsProcessTrustedWithOptions(opts) != 0;
            CFRelease(opts);
            let listen = IOHIDRequestAccess(HID_REQUEST_LISTEN_EVENT);
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            return Err(format!(
                "cannot watch the keyboard yet (Accessibility: {}, Input Monitoring: {}).\n  -> System Settings > Privacy & Security: switch on {exe} under\n     Accessibility AND Input Monitoring (or the terminal app running it).",
                if trusted { "on" } else { "off" },
                if listen { "on" } else { "off" }
            ));
        }
        TAP.store(tap, Relaxed);
        let src = CFMachPortCreateRunLoopSource(null_mut(), tap, 0);
        CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        CFRunLoopRun();
        CFRelease(src);
    }
    Ok(())
}

pub fn screen_bounds() -> Option<(f64, f64, f64, f64)> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    unsafe {
        if CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut n) != 0 || n == 0 {
            return None;
        }
        let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &d in &ids[..n as usize] {
            let r = CGDisplayBounds(d);
            b.0 = b.0.min(r.origin.x);
            b.1 = b.1.min(r.origin.y);
            b.2 = b.2.max(r.origin.x + r.size.width);
            b.3 = b.3.max(r.origin.y + r.size.height);
        }
        Some(b)
    }
}

pub fn set_remote(on: bool) {
    // Freeze the local cursor while it drives the other computer; deltas keep flowing.
    unsafe {
        CGAssociateMouseAndMouseCursorPosition(!on as u32);
    }
}

pub fn smooth_scale(cfg: &Config) -> f32 {
    cfg.scroll_pixels
}

fn cursor() -> CGPoint {
    unsafe {
        let e = CGEventCreate(null_mut());
        if e.is_null() {
            return CGPoint::default();
        }
        let p = CGEventGetLocation(e);
        CFRelease(e);
        p
    }
}

pub struct Injector {
    src: Ref,
    flags: u64,
    mods_held: u16,
    buttons: u8,
    px_per_notch: f32,
    acc: (f32, f32),
    last_click: (Instant, u8, i64),
}

// The event source is only touched while holding the engine's injector mutex.
unsafe impl Send for Injector {}

impl Injector {
    pub fn new(cfg: &Config) -> Result<Injector, String> {
        let src = unsafe { CGEventSourceCreate(1) }; // HID system state
        if src.is_null() {
            return Err("CGEventSourceCreate failed".into());
        }
        Ok(Injector {
            src,
            flags: 0,
            mods_held: 0,
            buttons: 0,
            px_per_notch: cfg.scroll_pixels,
            acc: (0.0, 0.0),
            last_click: (Instant::now(), 0, 0),
        })
    }

    unsafe fn post(&self, e: Ref) {
        if e.is_null() {
            return;
        }
        CGEventSetIntegerValueField(e, F_USER_DATA, MAGIC);
        CGEventPost(HID_TAP, e);
        CFRelease(e);
    }

    pub fn key(&mut self, hid: u16, state: u8) {
        if let Some(nx) = keys::to_mac_nx(hid) {
            unsafe { post_aux(nx, state != 0) };
            return;
        }
        let Some(code) = keys::to_mac(hid) else { return };
        let mf = mac_mod_flags(code);
        unsafe {
            if mf != 0 && code != 0x39 {
                let bit = 1u16 << (code - 0x36);
                if state == 0 {
                    self.mods_held &= !bit;
                } else {
                    self.mods_held |= bit;
                }
                self.flags = (0x36..=0x3Eu16)
                    .filter(|c| self.mods_held & (1 << (c - 0x36)) != 0)
                    .fold(0, |f, c| f | mac_mod_flags(c));
                let e = CGEventCreateKeyboardEvent(self.src, code, state != 0);
                if !e.is_null() {
                    CGEventSetType(e, FLAGS_CHANGED);
                    CGEventSetFlags(e, self.flags | NX_NONCOALESCED);
                }
                self.post(e);
                return;
            }
            let e = CGEventCreateKeyboardEvent(self.src, code, state != 0);
            if !e.is_null() {
                CGEventSetFlags(e, self.flags | NX_NONCOALESCED);
                if state == 2 {
                    CGEventSetIntegerValueField(e, F_AUTOREPEAT, 1);
                }
            }
            self.post(e);
        }
    }

    pub fn motion(&mut self, dx: i32, dy: i32) {
        let p = cursor();
        let (x0, y0, x1, y1) = screen_bounds().unwrap_or((0.0, 0.0, 1e5, 1e5));
        let np = CGPoint { x: (p.x + dx as f64).clamp(x0, x1 - 1.0), y: (p.y + dy as f64).clamp(y0, y1 - 1.0) };
        let (ty, btn) = if self.buttons & 1 != 0 {
            (LEFT_DRAGGED, 0)
        } else if self.buttons & 2 != 0 {
            (RIGHT_DRAGGED, 1)
        } else if self.buttons != 0 {
            (OTHER_DRAGGED, 2)
        } else {
            (MOUSE_MOVED, 0)
        };
        unsafe {
            let e = CGEventCreateMouseEvent(self.src, ty, np, btn);
            if !e.is_null() {
                CGEventSetIntegerValueField(e, F_DELTA_X, dx as i64);
                CGEventSetIntegerValueField(e, F_DELTA_Y, dy as i64);
                CGEventSetFlags(e, self.flags);
            }
            self.post(e);
        }
    }

    pub fn button(&mut self, b: u8, down: bool) {
        if !(1..=8).contains(&b) {
            return;
        }
        let num = (b - 1) as u32;
        let ty = match (b, down) {
            (1, true) => LEFT_DOWN,
            (1, false) => LEFT_UP,
            (2, true) => RIGHT_DOWN,
            (2, false) => RIGHT_UP,
            (_, true) => OTHER_DOWN,
            (_, false) => OTHER_UP,
        };
        let bit = 1u8 << (b - 1);
        if down {
            self.buttons |= bit;
            let (t, lb, n) = self.last_click;
            let count = if lb == b && t.elapsed().as_millis() < 400 { n + 1 } else { 1 };
            self.last_click = (Instant::now(), b, count);
        } else {
            self.buttons &= !bit;
        }
        unsafe {
            let e = CGEventCreateMouseEvent(self.src, ty, cursor(), num);
            if !e.is_null() {
                CGEventSetIntegerValueField(e, F_BUTTON_NUMBER, num as i64);
                CGEventSetIntegerValueField(e, F_CLICK_STATE, self.last_click.2.max(1));
                CGEventSetFlags(e, self.flags);
            }
            self.post(e);
        }
    }

    /// 1/120-notch units -> pixel scroll.
    pub fn wheel(&mut self, dx: i32, dy: i32) {
        let k = self.px_per_notch / 120.0;
        self.acc.0 += dx as f32 * k;
        self.acc.1 += dy as f32 * k;
        let (px, py) = (self.acc.0.trunc(), self.acc.1.trunc());
        self.acc.0 -= px;
        self.acc.1 -= py;
        if px != 0.0 || py != 0.0 {
            self.smooth_scroll(px as i32, py as i32);
        }
    }

    /// Native pixel units.
    pub fn smooth_scroll(&mut self, dx: i32, dy: i32) {
        unsafe {
            let e = CGEventCreateScrollWheelEvent2(self.src, SCROLL_PIXEL, 2, dy, -dx, 0);
            self.post(e);
        }
    }

    pub fn warp(&mut self, edge: Edge, ratio: f32) {
        let Some((x0, y0, x1, y1)) = screen_bounds() else { return };
        let r = ratio as f64;
        let p = match edge {
            Edge::Left => CGPoint { x: x0 + 2.0, y: y0 + r * (y1 - y0) },
            Edge::Right => CGPoint { x: x1 - 3.0, y: y0 + r * (y1 - y0) },
            Edge::Top => CGPoint { x: x0 + r * (x1 - x0), y: y0 + 2.0 },
            Edge::Bottom => CGPoint { x: x0 + r * (x1 - x0), y: y1 - 3.0 },
        };
        unsafe {
            CGWarpMouseCursorPosition(p);
        }
    }

    pub fn edge_ratio(&mut self, edge: Edge) -> Option<f32> {
        let p = cursor();
        at_edge(edge, screen_bounds()?, p.x, p.y)
    }
}
