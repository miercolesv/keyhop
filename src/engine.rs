//! Platform-neutral state machine: hotkeys, app launching, edge switching,
//! forwarding captured input to the peer and injecting the peer's input.

use crate::config::{Config, Edge};
use crate::keys::{self, KeySet};
use crate::net::Channel;
use crate::platform;
use crate::proto::Msg;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Wheel values are in 1/120 of a notch.
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Key { hid: u16, down: bool, repeat: bool },
    /// `pos` = absolute cursor position when the platform knows it.
    Motion { dx: i32, dy: i32, pos: Option<(f64, f64)> },
    Button { button: u8, down: bool },
    /// `continuous` = trackpad / already-smooth source (never re-smoothed).
    Wheel { dx: i32, dy: i32, continuous: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Swallow,
}

#[allow(dead_code)]
pub trait Handler: Send + Sync {
    fn on_input(&self, ev: &Input) -> Verdict;
    /// Whether any physical key is currently held (Linux delays keyboard grabs until none are).
    fn keys_held(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Local,
    Remote,
    Controlled,
}

pub struct Link {
    pub id: u64,
    pub tx: Sender<Msg>,
    pub chan: Arc<Channel>,
    pub preferred: bool,
}

struct St {
    mode: Mode,
    held: KeySet,
    forwarded: KeySet,
    forwarded_buttons: u8,
    pass_ups: KeySet,
    swallow_ups: KeySet,
    injected: KeySet,
    injected_buttons: u8,
    link: Option<Link>,
    remote_edge: Edge,
    return_edge: Option<Edge>,
    bounds: Option<(f64, f64, f64, f64)>,
    bounds_at: Instant,
    acc: (i32, i32),
}

pub struct Engine {
    cfg: Config,
    st: Mutex<St>,
    inj: Arc<Mutex<platform::Injector>>,
    smoother: Option<Mutex<Sender<(f32, f32)>>>,
    lights: Option<Mutex<Sender<crate::lights::Msg>>>,
    next_id: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn mods_of(held: &KeySet, except: u16) -> u8 {
    (0xE0u16..=0xE7).filter(|&k| k != except && held.contains(k)).fold(0, |m, k| m | keys::mod_bit(k))
}

impl Engine {
    pub fn new(cfg: Config) -> Result<Arc<Engine>, String> {
        let inj = Arc::new(Mutex::new(platform::Injector::new(&cfg)?));
        let smoother = cfg.smooth_scroll.then(|| {
            let i = inj.clone();
            let scale = platform::smooth_scale(&cfg);
            Mutex::new(crate::smooth::spawn(cfg.scroll_ms, scale, move |dx, dy| {
                lock(&i).smooth_scroll(dx, dy)
            }))
        });
        let accent_menu = cfg.accent_lights && cfg!(target_os = "macos");
        if cfg.accent_lights && !accent_menu {
            crate::log!("accent_lights: only macOS has the press-and-hold accent menu; ignored");
        }
        let hints = cfg.terminal_hints && cfg!(unix);
        if cfg.terminal_hints && !hints {
            crate::log!("terminal_hints: needs a Unix shell (zsh); ignored");
        }
        let lights = (accent_menu || hints).then(|| crate::lights::spawn(accent_menu));
        #[cfg(unix)]
        if let (true, Some(tx)) = (hints, &lights) {
            crate::hints::serve(tx.clone())?;
        }
        let lights = lights.map(Mutex::new);
        Ok(Arc::new(Engine {
            st: Mutex::new(St {
                mode: Mode::Local,
                held: KeySet::default(),
                forwarded: KeySet::default(),
                forwarded_buttons: 0,
                pass_ups: KeySet::default(),
                swallow_ups: KeySet::default(),
                injected: KeySet::default(),
                injected_buttons: 0,
                link: None,
                remote_edge: cfg.peer_edge.unwrap_or(Edge::Right),
                return_edge: None,
                bounds: None,
                bounds_at: Instant::now() - Duration::from_secs(60),
                acc: (0, 0),
            }),
            cfg,
            inj,
            smoother,
            lights,
            next_id: AtomicU64::new(1),
        }))
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    fn send(s: &St, m: Msg) {
        if let Some(l) = &s.link {
            let _ = l.tx.send(m);
        }
    }

    fn go_remote(&self, s: &mut St, edge: Edge, ratio: f32) {
        if s.mode != Mode::Local || s.link.is_none() {
            return;
        }
        s.mode = Mode::Remote;
        s.remote_edge = edge;
        s.pass_ups = s.held;
        for k in s.swallow_ups.iter().collect::<Vec<_>>() {
            s.pass_ups.remove(k);
        }
        s.forwarded.clear();
        s.forwarded_buttons = 0;
        s.acc = (0, 0);
        Self::send(s, Msg::Enter { edge: edge.to_u8(), ratio });
        platform::set_remote(true);
        crate::log!("control -> peer");
    }

    fn go_local(&self, s: &mut St, ratio: Option<f32>) {
        if s.mode != Mode::Remote {
            return;
        }
        for k in s.forwarded.iter().collect::<Vec<_>>() {
            Self::send(s, Msg::Key { hid: k, state: 0 });
        }
        for b in 1..=8u8 {
            if s.forwarded_buttons & (1 << (b - 1)) != 0 {
                Self::send(s, Msg::Button { button: b, down: false });
            }
        }
        s.forwarded.clear();
        s.forwarded_buttons = 0;
        s.mode = Mode::Local;
        platform::set_remote(false);
        if let Some(r) = ratio {
            lock(&self.inj).warp(s.remote_edge, r);
        }
        crate::log!("control -> local");
    }

    fn release_injected(&self, s: &mut St) {
        let mut inj = lock(&self.inj);
        for k in s.injected.iter().collect::<Vec<_>>() {
            inj.key(k, 0);
        }
        for b in 1..=8u8 {
            if s.injected_buttons & (1 << (b - 1)) != 0 {
                inj.button(b, false);
            }
        }
        s.injected.clear();
        s.injected_buttons = 0;
    }

    fn launch(argv: &[String]) {
        let argv = argv.to_vec();
        std::thread::spawn(move || {
            let mut cmd = std::process::Command::new(&argv[0]);
            cmd.args(&argv[1..])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            match cmd.spawn() {
                Ok(mut c) => {
                    let _ = c.wait();
                }
                Err(e) => crate::log!("launch {:?} failed: {e}", argv[0]),
            }
        });
    }

    fn screen(&self, s: &mut St) -> Option<(f64, f64, f64, f64)> {
        if s.bounds_at.elapsed() > Duration::from_secs(2) {
            s.bounds = platform::screen_bounds();
            s.bounds_at = Instant::now();
        }
        s.bounds
    }

    // ---------- network side ----------

    pub fn new_link_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Install a session. Returns false if an existing preferred session wins.
    pub fn attach(&self, link: Link) -> bool {
        let mut s = lock(&self.st);
        if let Some(old) = &s.link {
            if old.preferred && !link.preferred {
                return false;
            }
            old.chan.shutdown();
        }
        crate::log!("peer connected");
        s.link = Some(link);
        true
    }

    pub fn detach(&self, id: u64) {
        let mut s = lock(&self.st);
        if s.link.as_ref().map(|l| l.id) != Some(id) {
            return;
        }
        s.link = None;
        match s.mode {
            Mode::Remote => self.go_local(&mut s, None),
            Mode::Controlled => {
                self.release_injected(&mut s);
                s.mode = Mode::Local;
            }
            Mode::Local => {}
        }
        crate::log!("peer disconnected");
    }

    pub fn has_link(&self) -> bool {
        lock(&self.st).link.is_some()
    }

    pub fn on_remote(&self, m: Msg) {
        let mut s = lock(&self.st);
        match m {
            Msg::Enter { edge, ratio } => {
                if s.mode == Mode::Remote {
                    self.go_local(&mut s, None);
                }
                let entry = Edge::from_u8(edge).map(Edge::opposite);
                s.mode = Mode::Controlled;
                s.return_edge = entry;
                if let Some(e) = entry {
                    lock(&self.inj).warp(e, ratio.clamp(0.0, 1.0));
                }
            }
            Msg::Leave { ratio } => {
                if s.mode == Mode::Remote {
                    self.go_local(&mut s, Some(ratio.clamp(0.0, 1.0)));
                }
            }
            _ if s.mode != Mode::Controlled => {}
            Msg::Key { hid, state } => {
                if state == 0 {
                    s.injected.remove(hid);
                } else {
                    s.injected.insert(hid);
                }
                lock(&self.inj).key(hid, state);
            }
            Msg::Button { button, down } => {
                if (1..=8).contains(&button) {
                    let bit = 1 << (button - 1);
                    if down {
                        s.injected_buttons |= bit
                    } else {
                        s.injected_buttons &= !bit
                    }
                    lock(&self.inj).button(button, down);
                }
            }
            Msg::Wheel { dx, dy } => lock(&self.inj).wheel(dx as i32, dy as i32),
            Msg::Motion { dx, dy } => {
                let (dx, dy) = (dx as i32, dy as i32);
                let ret = {
                    let mut inj = lock(&self.inj);
                    inj.motion(dx, dy);
                    match s.return_edge {
                        Some(e) if toward(e, dx, dy) && s.injected_buttons == 0 => inj.edge_ratio(e),
                        _ => None,
                    }
                };
                if let Some(r) = ret {
                    self.release_injected(&mut s);
                    s.mode = Mode::Local;
                    Self::send(&s, Msg::Leave { ratio: r });
                }
            }
            _ => {}
        }
    }
}

fn toward(e: Edge, dx: i32, dy: i32) -> bool {
    match e {
        Edge::Left => dx < 0,
        Edge::Right => dx > 0,
        Edge::Top => dy < 0,
        Edge::Bottom => dy > 0,
    }
}

/// If (x, y) sits on edge `e` of `b`, the position along that edge (0..1).
pub fn at_edge(e: Edge, b: (f64, f64, f64, f64), x: f64, y: f64) -> Option<f32> {
    let (x0, y0, x1, y1) = b;
    let hit = match e {
        Edge::Left => x <= x0 + 0.5,
        Edge::Right => x >= x1 - 1.5,
        Edge::Top => y <= y0 + 0.5,
        Edge::Bottom => y >= y1 - 1.5,
    };
    hit.then(|| {
        let r = match e {
            Edge::Left | Edge::Right => (y - y0) / (y1 - y0).max(1.0),
            _ => (x - x0) / (x1 - x0).max(1.0),
        };
        r.clamp(0.0, 1.0) as f32
    })
}

impl Handler for Engine {
    fn on_input(&self, ev: &Input) -> Verdict {
        let mut s = lock(&self.st);
        match *ev {
            Input::Key { hid, down, repeat } => {
                let repeat = down && (repeat || s.held.contains(hid));
                if down {
                    s.held.insert(hid);
                } else {
                    s.held.remove(hid);
                    if s.swallow_ups.remove(hid) {
                        return Verdict::Swallow;
                    }
                    if s.pass_ups.remove(hid) {
                        return Verdict::Pass;
                    }
                }
                let mods = mods_of(&s.held, hid);
                if down {
                    if let Some(hk) = self.cfg.switch_hotkey {
                        if hk.hid == hid && hk.mods == mods {
                            if !repeat {
                                s.swallow_ups.insert(hid);
                                match s.mode {
                                    Mode::Local => {
                                        let e = self.cfg.peer_edge.unwrap_or(Edge::Right);
                                        self.go_remote(&mut s, e, 0.5)
                                    }
                                    Mode::Remote => self.go_local(&mut s, Some(0.5)),
                                    Mode::Controlled => {}
                                }
                            }
                            return Verdict::Swallow;
                        }
                    }
                }
                if s.mode == Mode::Remote {
                    if down {
                        s.forwarded.insert(hid);
                    } else if !s.forwarded.remove(hid) {
                        return Verdict::Swallow;
                    }
                    Self::send(&s, Msg::Key { hid, state: if !down { 0 } else if repeat { 2 } else { 1 } });
                    return Verdict::Swallow;
                }
                if let Some(l) = &self.lights {
                    if !repeat {
                        let plain = mods & !keys::MOD_SHIFT == 0;
                        let _ = lock(l).send(crate::lights::Msg::Key { hid, down, plain });
                    }
                }
                if down {
                    if let Some(b) = self.cfg.bindings.iter().find(|b| b.hotkey.hid == hid && b.hotkey.mods == mods) {
                        if !repeat {
                            s.swallow_ups.insert(hid);
                            Self::launch(&b.argv);
                        }
                        return Verdict::Swallow;
                    }
                }
                Verdict::Pass
            }
            Input::Motion { dx, dy, pos } => match s.mode {
                Mode::Remote => {
                    // Accumulate sub-frame motion; i16 range is plenty per message.
                    let (ax, ay) = (s.acc.0 + dx, s.acc.1 + dy);
                    let (sx, sy) = (ax.clamp(-32768, 32767), ay.clamp(-32768, 32767));
                    s.acc = (ax - sx, ay - sy);
                    if sx != 0 || sy != 0 {
                        Self::send(&s, Msg::Motion { dx: sx as i16, dy: sy as i16 });
                    }
                    Verdict::Swallow
                }
                Mode::Local => {
                    if let (Some(e), Some((x, y)), true) = (self.cfg.peer_edge, pos, s.link.is_some()) {
                        if toward(e, dx, dy) {
                            if let Some(b) = self.screen(&mut s) {
                                if let Some(r) = at_edge(e, b, x, y) {
                                    self.go_remote(&mut s, e, r);
                                    return Verdict::Swallow;
                                }
                            }
                        }
                    }
                    Verdict::Pass
                }
                Mode::Controlled => Verdict::Pass,
            },
            Input::Button { button, down } => {
                if s.mode == Mode::Remote {
                    let bit = 1u8.checked_shl(button.saturating_sub(1) as u32).unwrap_or(0);
                    if down {
                        s.forwarded_buttons |= bit
                    } else {
                        s.forwarded_buttons &= !bit
                    }
                    Self::send(&s, Msg::Button { button, down });
                    Verdict::Swallow
                } else {
                    Verdict::Pass
                }
            }
            Input::Wheel { dx, dy, continuous } => {
                if s.mode == Mode::Remote {
                    let c = |v: i32| v.clamp(-32768, 32767) as i16;
                    Self::send(&s, Msg::Wheel { dx: c(dx), dy: c(dy) });
                    return Verdict::Swallow;
                }
                if !continuous {
                    if let Some(sm) = &self.smoother {
                        let _ = lock(sm).send((dx as f32 / 120.0, dy as f32 / 120.0));
                        return Verdict::Swallow;
                    }
                }
                Verdict::Pass
            }
        }
    }

    fn keys_held(&self) -> bool {
        let s = lock(&self.st);
        !s.held.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges() {
        let b = (0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(at_edge(Edge::Right, b, 1919.0, 540.0), Some(0.5));
        assert_eq!(at_edge(Edge::Right, b, 1000.0, 540.0), None);
        assert_eq!(at_edge(Edge::Left, b, 0.0, 1080.0), Some(1.0));
        assert_eq!(at_edge(Edge::Top, b, 480.0, 0.0), Some(0.25));
        assert!(toward(Edge::Bottom, 0, 3) && !toward(Edge::Bottom, 0, -3));
    }

    #[test]
    fn mods() {
        let mut h = KeySet::default();
        h.insert(0xE0);
        h.insert(0xE6);
        h.insert(0x68);
        assert_eq!(mods_of(&h, 0x68), keys::MOD_CTRL | keys::MOD_ALT);
        assert_eq!(mods_of(&h, 0xE0), keys::MOD_ALT);
    }
}
