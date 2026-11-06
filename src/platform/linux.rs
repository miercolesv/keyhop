//! Linux backend: evdev for capture, uinput for injection. Works the same on
//! X11 and Wayland. Needs read access to /dev/input/event* and write access to
//! /dev/uinput (see README: `input` group + udev rule).
//!
//! Devices are only grabbed (exclusive) when necessary: mice while smooth
//! scrolling is on or while controlling the peer, keyboards only while
//! controlling the peer.

use crate::config::{Config, Edge};
use crate::engine::{Handler, Input, Verdict};
use crate::keys;
use evdev::raw_stream::RawDevice;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, InputEvent, KeyCode, RelativeAxisCode};
use std::collections::HashSet;
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const EV_REL: u16 = 2;
const REL_X: u16 = 0;
const REL_Y: u16 = 1;
const REL_HWHEEL: u16 = 6;
const REL_WHEEL: u16 = 8;
const REL_WHEEL_HI_RES: u16 = 11;
const REL_HWHEEL_HI_RES: u16 = 12;
const BTN_LEFT: u16 = 0x110;
const BTN_TASK: u16 = 0x117;
const VIRT_PREFIX: &str = "keyhop virtual";
const EVIOCGRAB: u64 = 0x4004_4590;

static REMOTE: AtomicBool = AtomicBool::new(false);
static GRAB_MICE: AtomicBool = AtomicBool::new(false);
static PENDING_KBD: AtomicBool = AtomicBool::new(false);
static KBD_GRABBED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct Registry {
    kbds: Vec<RawFd>,
    mice: Vec<RawFd>,
}
static REG: Mutex<Registry> = Mutex::new(Registry { kbds: Vec::new(), mice: Vec::new() });

fn reg() -> std::sync::MutexGuard<'static, Registry> {
    REG.lock().unwrap_or_else(|p| p.into_inner())
}

fn grab_fd(fd: RawFd, on: bool) {
    // SAFETY: plain ioctl on a descriptor we own; failure is harmless.
    unsafe {
        libc::ioctl(fd, EVIOCGRAB as _, on as libc::c_int);
    }
}

// ---------------------------------------------------------------- virtual devices

struct Virt {
    kbd: VirtualDevice,
    ptr: VirtualDevice,
    acc: (i32, i32),
}

static VIRT: OnceLock<Mutex<Virt>> = OnceLock::new();

fn virt() -> Result<&'static Mutex<Virt>, String> {
    if let Some(v) = VIRT.get() {
        return Ok(v);
    }
    let build = || -> std::io::Result<Virt> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for c in 1..0x100u16 {
            keys.insert(KeyCode::new(c));
        }
        let kbd = VirtualDevice::builder()?.name(&format!("{VIRT_PREFIX} keyboard")).with_keys(&keys)?.build()?;
        let mut btns = AttributeSet::<KeyCode>::new();
        for c in BTN_LEFT..=BTN_TASK {
            btns.insert(KeyCode::new(c));
        }
        let mut rel = AttributeSet::<RelativeAxisCode>::new();
        for r in [REL_X, REL_Y, REL_WHEEL, REL_HWHEEL, REL_WHEEL_HI_RES, REL_HWHEEL_HI_RES] {
            rel.insert(RelativeAxisCode(r));
        }
        let ptr = VirtualDevice::builder()?
            .name(&format!("{VIRT_PREFIX} pointer"))
            .with_keys(&btns)?
            .with_relative_axes(&rel)?
            .build()?;
        Ok(Virt { kbd, ptr, acc: (0, 0) })
    };
    let v = build().map_err(|e| format!("cannot create uinput device: {e}\n  -> see README: add yourself to the `input` group and install the udev rule for /dev/uinput"))?;
    let _ = VIRT.set(Mutex::new(v));
    Ok(VIRT.get().unwrap())
}

fn with_virt(f: impl FnOnce(&mut Virt)) {
    if let Some(v) = VIRT.get() {
        f(&mut v.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

fn ev(t: u16, c: u16, v: i32) -> InputEvent {
    InputEvent::new(t, c, v)
}

impl Virt {
    /// Hi-res wheel units (120/notch) plus legacy notches once a full one accumulates.
    fn wheel(&mut self, dx: i32, dy: i32) {
        let mut out = Vec::with_capacity(4);
        if dy != 0 {
            out.push(ev(EV_REL, REL_WHEEL_HI_RES, dy));
            self.acc.1 += dy;
            let n = self.acc.1 / 120;
            if n != 0 {
                out.push(ev(EV_REL, REL_WHEEL, n));
                self.acc.1 -= n * 120;
            }
        }
        if dx != 0 {
            out.push(ev(EV_REL, REL_HWHEEL_HI_RES, dx));
            self.acc.0 += dx;
            let n = self.acc.0 / 120;
            if n != 0 {
                out.push(ev(EV_REL, REL_HWHEEL, n));
                self.acc.0 -= n * 120;
            }
        }
        if !out.is_empty() {
            let _ = self.ptr.emit(&out);
        }
    }
}

// ---------------------------------------------------------------- injector

pub struct Injector;

impl Injector {
    pub fn new(_cfg: &Config) -> Result<Injector, String> {
        virt().map(|_| Injector)
    }
    pub fn key(&mut self, hid: u16, state: u8) {
        if let Some(c) = keys::to_evdev(hid) {
            with_virt(|v| {
                let _ = v.kbd.emit(&[ev(EV_KEY, c, state as i32)]);
            });
        }
    }
    pub fn motion(&mut self, dx: i32, dy: i32) {
        with_virt(|v| {
            let _ = v.ptr.emit(&[ev(EV_REL, REL_X, dx), ev(EV_REL, REL_Y, dy)]);
        });
    }
    pub fn button(&mut self, b: u8, down: bool) {
        let code = match b {
            1 => 0x110,
            2 => 0x111,
            3 => 0x112,
            4 => 0x113, // BTN_SIDE (back)
            5 => 0x114, // BTN_EXTRA (forward)
            _ => return,
        };
        with_virt(|v| {
            let _ = v.ptr.emit(&[ev(EV_KEY, code, down as i32)]);
        });
    }
    pub fn wheel(&mut self, dx: i32, dy: i32) {
        with_virt(|v| v.wheel(dx, dy));
    }
    pub fn smooth_scroll(&mut self, dx: i32, dy: i32) {
        with_virt(|v| v.wheel(dx, dy));
    }
    /// Wayland exposes no global cursor position: edge handling uses the hotkey.
    pub fn warp(&mut self, _e: Edge, _ratio: f32) {}
    pub fn edge_ratio(&mut self, _e: Edge) -> Option<f32> {
        None
    }
}

pub fn smooth_scale(_cfg: &Config) -> f32 {
    120.0
}

pub fn screen_bounds() -> Option<(f64, f64, f64, f64)> {
    None
}

pub fn set_remote(on: bool) {
    REMOTE.store(on, SeqCst);
    let r = reg();
    if on {
        for &fd in &r.mice {
            grab_fd(fd, true);
        }
        PENDING_KBD.store(true, SeqCst); // grabbed once all keys are released
    } else {
        PENDING_KBD.store(false, SeqCst);
        if KBD_GRABBED.swap(false, SeqCst) {
            for &fd in &r.kbds {
                grab_fd(fd, false);
            }
        }
        if !GRAB_MICE.load(SeqCst) {
            for &fd in &r.mice {
                grab_fd(fd, false);
            }
        }
    }
}

// ---------------------------------------------------------------- capture

fn btn_number(code: u16) -> Option<u8> {
    Some(match code {
        0x110 => 1,
        0x111 => 2,
        0x112 => 3,
        0x113 | 0x116 => 4, // side / back
        0x114 | 0x115 => 5, // extra / forward
        _ => return None,
    })
}

struct DevInfo {
    kbd: bool,
    mouse: bool,
    hires: bool,
}

fn classify(d: &RawDevice) -> Option<DevInfo> {
    if d.name().unwrap_or("").starts_with(VIRT_PREFIX) {
        return None;
    }
    let keys = d.supported_keys();
    let kbd = keys.is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_ENTER))
        || keys.is_some_and(|k| k.contains(KeyCode::KEY_F13)) // macro pads / extra key nodes
        // Media keys/knob arrive on a separate "Consumer Control" node.
        || keys.is_some_and(|k| k.contains(KeyCode::KEY_VOLUMEUP) || k.contains(KeyCode::KEY_PLAYPAUSE));
    let rel = d.supported_relative_axes();
    let mouse = rel.is_some_and(|r| r.contains(RelativeAxisCode::REL_X) || r.contains(RelativeAxisCode::REL_WHEEL))
        && keys.is_some_and(|k| k.contains(KeyCode::BTN_LEFT) || k.contains(KeyCode::BTN_MIDDLE));
    let hires = rel.is_some_and(|r| r.contains(RelativeAxisCode::REL_WHEEL_HI_RES));
    (kbd || mouse).then_some(DevInfo { kbd, mouse, hires })
}

fn reader(mut dev: RawDevice, info: DevInfo, h: Arc<dyn Handler>) {
    let fd = dev.as_raw_fd();
    {
        let mut r = reg();
        if info.kbd {
            r.kbds.push(fd);
            if KBD_GRABBED.load(SeqCst) {
                grab_fd(fd, true);
            }
        }
        if info.mouse {
            r.mice.push(fd);
            if GRAB_MICE.load(SeqCst) || REMOTE.load(SeqCst) {
                grab_fd(fd, true);
            }
        }
    }
    let grabbed = || {
        (info.mouse && (GRAB_MICE.load(SeqCst) || REMOTE.load(SeqCst))) || (info.kbd && KBD_GRABBED.load(SeqCst))
    };
    // Per-frame accumulators (flushed on SYN_REPORT).
    let mut items: Vec<(Input, InputEvent)> = Vec::with_capacity(8);
    let mut raw_other: Vec<InputEvent> = Vec::new();
    let (mut mx, mut my, mut wx, mut wy) = (0i32, 0i32, 0i32, 0i32);
    let mut kbd_out: Vec<InputEvent> = Vec::with_capacity(8);
    let mut ptr_out: Vec<InputEvent> = Vec::with_capacity(8);
    'outer: loop {
        let events: Vec<InputEvent> = match dev.fetch_events() {
            Ok(it) => it.collect(),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break 'outer,
        };
        for e in events {
            let (t, c, v) = (e.event_type().0, e.code(), e.value());
            match t {
                EV_KEY => {
                    if let Some(b) = btn_number(c) {
                        if v != 2 {
                            items.push((Input::Button { button: b, down: v != 0 }, e));
                        }
                    } else if let Some(hid) = keys::from_evdev(c) {
                        items.push((Input::Key { hid, down: v != 0, repeat: v == 2 }, e));
                    } else {
                        raw_other.push(e);
                    }
                }
                EV_REL => match c {
                    REL_X => mx += v,
                    REL_Y => my += v,
                    REL_WHEEL if !info.hires => wy += v * 120,
                    REL_HWHEEL if !info.hires => wx += v * 120,
                    REL_WHEEL_HI_RES => wy += v,
                    REL_HWHEEL_HI_RES => wx += v,
                    REL_WHEEL | REL_HWHEEL => {} // duplicate of hi-res stream
                    _ => raw_other.push(e),
                },
                EV_SYN if c == 0 => {
                    let g = grabbed();
                    for (inp, raw) in items.drain(..) {
                        if h.on_input(&inp) == Verdict::Pass && g {
                            if raw.code() < 0x100 { kbd_out.push(raw) } else { ptr_out.push(raw) }
                        }
                    }
                    if mx != 0 || my != 0 {
                        let inp = Input::Motion { dx: mx, dy: my, pos: None };
                        if h.on_input(&inp) == Verdict::Pass && g {
                            ptr_out.push(ev(EV_REL, REL_X, mx));
                            ptr_out.push(ev(EV_REL, REL_Y, my));
                        }
                    }
                    let (pwx, pwy) = (wx, wy);
                    if wx != 0 || wy != 0 {
                        let inp = Input::Wheel { dx: wx, dy: wy, continuous: false };
                        if h.on_input(&inp) == Verdict::Pass && g {
                            with_virt(|vd| vd.wheel(pwx, pwy));
                        }
                    }
                    if g {
                        ptr_out.append(&mut raw_other);
                    }
                    raw_other.clear();
                    (mx, my, wx, wy) = (0, 0, 0, 0);
                    if !kbd_out.is_empty() || !ptr_out.is_empty() {
                        with_virt(|vd| {
                            if !kbd_out.is_empty() {
                                let _ = vd.kbd.emit(&kbd_out);
                            }
                            if !ptr_out.is_empty() {
                                let _ = vd.ptr.emit(&ptr_out);
                            }
                        });
                        kbd_out.clear();
                        ptr_out.clear();
                    }
                    if info.kbd && PENDING_KBD.load(SeqCst) && !h.keys_held() && REMOTE.load(SeqCst) {
                        PENDING_KBD.store(false, SeqCst);
                        KBD_GRABBED.store(true, SeqCst);
                        for &fd in &reg().kbds {
                            grab_fd(fd, true);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut r = reg();
    r.kbds.retain(|&x| x != fd);
    r.mice.retain(|&x| x != fd);
}

fn scan(known: &'static Mutex<HashSet<PathBuf>>, h: &Arc<dyn Handler>, first: bool) -> usize {
    let Ok(rd) = std::fs::read_dir("/dev/input") else { return 0 };
    let mut denied = 0;
    for p in rd.flatten().map(|e| e.path()) {
        let is_event = p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("event"));
        if !is_event || known.lock().unwrap().contains(&p) {
            continue;
        }
        match RawDevice::open(&p) {
            Ok(d) => {
                known.lock().unwrap().insert(p.clone());
                if let Some(info) = classify(&d) {
                    crate::log!(
                        "using {} ({}{})",
                        d.name().unwrap_or("?"),
                        if info.kbd { "keyboard" } else { "" },
                        if info.mouse { if info.kbd { "+mouse" } else { "mouse" } } else { "" }
                    );
                    let h = h.clone();
                    let path = p.clone();
                    std::thread::spawn(move || {
                        reader(d, info, h);
                        known.lock().unwrap().remove(&path);
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                if first {
                    denied += 1
                }
            }
            Err(_) => {}
        }
    }
    denied
}

/// Blocks forever, scanning for (hot-plugged) devices every 2 s.
pub fn run_capture(h: Arc<dyn Handler>, grab_mice: bool) -> Result<(), String> {
    virt()?;
    GRAB_MICE.store(grab_mice, SeqCst);
    let known: &'static Mutex<HashSet<PathBuf>> = Box::leak(Box::new(Mutex::new(HashSet::new())));
    let denied = scan(known, &h, true);
    if known.lock().unwrap().is_empty() && denied > 0 {
        return Err("no permission to read /dev/input/event* -> add yourself to the `input` group (see README) and log in again".into());
    }
    loop {
        std::thread::sleep(Duration::from_secs(2));
        scan(known, &h, false);
    }
}
