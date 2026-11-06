//! keyhop — lean cross-platform keyboard helper.
//! App-launch hotkeys, smooth scrolling, encrypted keyboard/mouse sharing and
//! file transfer between two computers. No web view, no telemetry.

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => {{
        let s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) % 86_400;
        eprintln!("[{:02}:{:02}:{:02}] {}", s / 3600, s / 60 % 60, s % 60, format!($($t)*));
    }};
}

mod accents;
mod config;
mod device;
mod engine;
mod files;
#[cfg(unix)]
mod hints;
mod keys;
mod lights;
mod net;
mod platform;
mod proto;
mod setup;
mod smooth;

use config::{config_path, Config};
use engine::{Handler, Input, Verdict};
use std::sync::Arc;

const USAGE: &str = "keyhop — app hotkeys, smooth scrolling, encrypted keyboard/mouse sharing

USAGE:
  keyhop setup                  guided first-time setup (start here)
  keyhop init                   create config + identity key
  keyhop run [--background]     run the service (foreground by default)
  keyhop keys                   print the name of each key/button you press
  keyhop pair --listen          wait for the other computer to pair
  keyhop pair <host:port>       pair with a computer running `pair --listen`
  keyhop trusted                list paired computers
  keyhop untrust <name>         remove a paired computer
  keyhop send <file>...         send files to the configured peer
  keyhop id                     show this computer's key fingerprint
  keyhop install-service        start keyhop automatically at login
";

fn load_cfg() -> Result<Config, String> {
    Config::load(&config_path())
}

fn identity() -> Result<net::Identity, String> {
    net::Identity::load_or_create().map_err(|e| format!("identity: {e}"))
}

struct KeyPrinter;
impl Handler for KeyPrinter {
    fn on_input(&self, ev: &Input) -> Verdict {
        match *ev {
            Input::Key { hid, down: true, repeat: false } if keys::is_consumer(hid) => {
                println!("media  {:<14} (consumer 0x{:03x})", keys::name(hid), hid & 0x0FFF)
            }
            Input::Key { hid, down: true, repeat: false } => println!("key    {:<14} (hid 0x{hid:02x})", keys::name(hid)),
            Input::Button { button, down: true } => println!("button {button}"),
            Input::Wheel { dx, dy, continuous } => {
                println!("wheel  dx={dx} dy={dy}{}", if continuous { " (smooth source)" } else { "" })
            }
            _ => {}
        }
        Verdict::Pass
    }
}

fn run(background: bool) -> Result<(), String> {
    #[cfg(windows)]
    if background {
        platform::detach_console();
    }
    let _ = background;
    let cfg = load_cfg()?;
    let id = Arc::new(identity()?);
    let trusted = net::Trusted::load();
    for b in &cfg.bindings {
        log!("bind {} -> {}", keys::hotkey_name(&b.hotkey), b.argv.join(" "));
    }
    if let Some(h) = &cfg.switch_hotkey {
        log!("switch hotkey {}", keys::hotkey_name(h));
    }
    let grab_mice = cfg.smooth_scroll;
    let share = cfg.peer.is_some() || !trusted.0.is_empty();
    let engine = engine::Engine::new(cfg)?;
    if share {
        net::serve(engine.clone(), id, trusted)?;
    } else {
        log!("no paired computer: keyboard/mouse sharing disabled");
    }
    platform::run_capture(engine, grab_mice)
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["init"] => {
            let p = config_path();
            if !p.exists() {
                std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(&p, config::TEMPLATE).map_err(|e| e.to_string())?;
                println!("created {}", p.display());
            } else {
                println!("{} already exists", p.display());
            }
            let id = identity()?;
            println!("identity {}  ({})", net::fingerprint(&id.public), net::Identity::path().display());
            Ok(())
        }
        ["run"] => run(false),
        ["run", "--background"] => run(true),
        ["keys"] => {
            println!("Press keys/buttons (Ctrl+C to quit)...");
            platform::run_capture(Arc::new(KeyPrinter), false)
        }
        ["pair", "--listen"] => net::pair(&load_cfg()?, &identity()?, None),
        ["pair", addr] => net::pair(&load_cfg()?, &identity()?, Some(addr)),
        ["trusted"] => {
            let t = net::Trusted::load();
            if t.0.is_empty() {
                println!("no paired computers");
            }
            for (k, n) in &t.0 {
                println!("{n:<24} {}", net::fingerprint(k));
            }
            Ok(())
        }
        ["untrust", name] => {
            let mut t = net::Trusted::load();
            let before = t.0.len();
            t.0.retain(|(k, n)| n != name && !net::fingerprint(k).starts_with(name));
            if t.0.len() == before {
                return Err(format!("no paired computer matches '{name}'"));
            }
            t.save().map_err(|e| e.to_string())
        }
        ["send", files @ ..] if !files.is_empty() => {
            let files: Vec<String> = files.iter().map(|s| s.to_string()).collect();
            files::send(&load_cfg()?, &identity()?, &files)
        }
        ["id"] => {
            println!("{}", net::fingerprint(&identity()?.public));
            Ok(())
        }
        ["install-service"] => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            setup::install_service(&exe).map(|s| println!("{s}"))
        }
        ["setup"] => setup::run(),
        ["device", rest @ ..] => device::main(rest),
        _ => {
            println!("{USAGE}{}\nConfig: {}", device::USAGE_TEXT, config_path().display());
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
