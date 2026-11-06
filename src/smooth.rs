//! Exponential-ease scroll smoother: turns wheel notches into a short stream of
//! small scroll steps. Runs on its own thread and sleeps when idle.

use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_micros(6_944); // ~144 Hz

/// `ms`: animation length. `scale`: native units emitted per notch
/// (120 for hi-res wheel units, or pixels per notch on macOS).
/// `emit(dx, dy)` receives integer native units.
pub fn spawn(ms: f32, scale: f32, emit: impl Fn(i32, i32) + Send + 'static) -> Sender<(f32, f32)> {
    let (tx, rx) = channel::<(f32, f32)>();
    let tau = (ms / 4.0).max(4.0) / 1000.0; // ~98% of distance covered after `ms`
    std::thread::Builder::new()
        .name("smooth-scroll".into())
        .spawn(move || {
            let (mut rem_x, mut rem_y, mut carry_x, mut carry_y) = (0f32, 0f32, 0f32, 0f32);
            let mut last = Instant::now();
            loop {
                let idle = rem_x.abs() < 0.5 && rem_y.abs() < 0.5;
                let msg = if idle {
                    carry_x = 0.0;
                    carry_y = 0.0;
                    match rx.recv() {
                        Ok(m) => Some(m),
                        Err(_) => return,
                    }
                } else {
                    match rx.recv_timeout(TICK) {
                        Ok(m) => Some(m),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                };
                if let Some((nx, ny)) = msg {
                    if idle {
                        last = Instant::now() - TICK;
                    }
                    // Reversing direction cancels the remaining glide.
                    if nx != 0.0 && nx.signum() != rem_x.signum() {
                        rem_x = 0.0;
                    }
                    if ny != 0.0 && ny.signum() != rem_y.signum() {
                        rem_y = 0.0;
                    }
                    rem_x += nx * scale;
                    rem_y += ny * scale;
                    while let Ok((nx, ny)) = rx.try_recv() {
                        rem_x += nx * scale;
                        rem_y += ny * scale;
                    }
                    if last.elapsed() < TICK {
                        continue;
                    }
                }
                let now = Instant::now();
                let dt = now.duration_since(last).as_secs_f32().min(0.05);
                last = now;
                let k = 1.0 - (-dt / tau).exp();
                let step = |rem: &mut f32| {
                    let s = if rem.abs() < 1.0 { *rem } else { *rem * k };
                    *rem -= s;
                    s
                };
                carry_x += step(&mut rem_x);
                carry_y += step(&mut rem_y);
                let (ox, oy) = (carry_x.trunc(), carry_y.trunc());
                carry_x -= ox;
                carry_y -= oy;
                if ox != 0.0 || oy != 0.0 {
                    emit(ox as i32, oy as i32);
                }
            }
        })
        .expect("spawn smoother");
    tx
}
