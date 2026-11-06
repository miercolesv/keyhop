//! Tiny line-based config parser (no serde/toml dependency).
//!
//! ```text
//! listen        = 0.0.0.0:24801
//! peer          = 192.168.1.20:24801
//! peer_edge     = right            # left | right | top | bottom | off
//! switch_hotkey = ctrl+alt+f19
//! receive_dir   = ~/Downloads/keyhop
//! smooth_scroll = on
//! scroll_pixels = 48               # macOS: pixels per wheel notch
//! scroll_ms     = 160
//! bind f13      = open -a Safari
//! ```

use crate::keys::{self, Hotkey};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    pub fn opposite(self) -> Edge {
        match self {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Top => Edge::Bottom,
            Edge::Bottom => Edge::Top,
        }
    }
    pub fn to_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(v: u8) -> Option<Edge> {
        Some(match v {
            0 => Edge::Left,
            1 => Edge::Right,
            2 => Edge::Top,
            3 => Edge::Bottom,
            _ => return None,
        })
    }
    fn parse(s: &str) -> Result<Option<Edge>, String> {
        Ok(Some(match s {
            "left" => Edge::Left,
            "right" => Edge::Right,
            "top" => Edge::Top,
            "bottom" => Edge::Bottom,
            "off" | "none" => return Ok(None),
            _ => return Err(format!("unknown edge '{s}'")),
        }))
    }
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub hotkey: Hotkey,
    pub argv: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub listen: String,
    pub peer: Option<String>,
    pub peer_edge: Option<Edge>,
    pub switch_hotkey: Option<Hotkey>,
    pub receive_dir: PathBuf,
    pub smooth_scroll: bool,
    pub accent_lights: bool,
    pub terminal_hints: bool,
    pub scroll_pixels: f32,
    pub scroll_ms: f32,
    pub bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: "0.0.0.0:24801".into(),
            peer: None,
            peer_edge: None,
            switch_hotkey: keys::parse_hotkey("ctrl+alt+f19").ok(),
            receive_dir: home().join("Downloads").join("keyhop"),
            smooth_scroll: false,
            accent_lights: false,
            terminal_hints: false,
            scroll_pixels: 48.0,
            scroll_ms: 160.0,
            bindings: Vec::new(),
        }
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    // Empty variables count as unset (XDG spec; the zsh hook does the same).
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    if cfg!(windows) {
        if let Some(a) = var("APPDATA") {
            return PathBuf::from(a).join("keyhop");
        }
    }
    if let Some(x) = var("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("keyhop");
    }
    home().join(".config").join("keyhop")
}

pub fn config_path() -> PathBuf {
    config_dir().join("keyhop.conf")
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
        home().join(rest)
    } else if s == "~" {
        home()
    } else {
        PathBuf::from(s)
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        _ => Err(format!("expected on/off, got '{s}'")),
    }
}

/// Split a command line into argv, honouring "double" and 'single' quotes.
pub fn split_args(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_tok = false;
    let mut quote: Option<char> = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => match chars.next() {
                Some(n @ ('"' | '\\')) => cur.push(n),
                Some(n) => {
                    cur.push('\\');
                    cur.push(n)
                }
                None => cur.push('\\'),
            },
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_tok = true
            }
            (None, c) if c.is_whitespace() => {
                if in_tok {
                    out.push(std::mem::take(&mut cur));
                    in_tok = false
                }
            }
            (None, c) => {
                cur.push(c);
                in_tok = true
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if in_tok {
        out.push(cur);
    }
    Ok(out)
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let mut c = Config::default();
        for (n, raw) in text.lines().enumerate() {
            let err = |e: String| format!("line {}: {e}", n + 1);
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| err("expected 'key = value'".into()))?;
            let (k, v) = (k.trim(), v.trim());
            if let Some(hk) = k.strip_prefix("bind ") {
                let hotkey = keys::parse_hotkey(hk.trim()).map_err(err)?;
                let argv = split_args(v).map_err(err)?;
                if argv.is_empty() {
                    return Err(err("empty command".into()));
                }
                c.bindings.push(Binding { hotkey, argv });
                continue;
            }
            match k {
                "listen" => c.listen = v.to_string(),
                "peer" => c.peer = (!v.is_empty()).then(|| v.to_string()),
                "peer_edge" => c.peer_edge = Edge::parse(v).map_err(err)?,
                "switch_hotkey" => {
                    c.switch_hotkey = if v == "off" {
                        None
                    } else {
                        Some(keys::parse_hotkey(v).map_err(err)?)
                    }
                }
                "receive_dir" => c.receive_dir = expand_tilde(v),
                "smooth_scroll" => c.smooth_scroll = parse_bool(v).map_err(err)?,
                "accent_lights" => c.accent_lights = parse_bool(v).map_err(err)?,
                "terminal_hints" => c.terminal_hints = parse_bool(v).map_err(err)?,
                "scroll_pixels" => {
                    c.scroll_pixels = v.parse().map_err(|_| err("bad number".into()))?
                }
                "scroll_ms" => c.scroll_ms = v.parse().map_err(|_| err("bad number".into()))?,
                _ => return Err(err(format!("unknown key '{k}'"))),
            }
        }
        c.scroll_ms = c.scroll_ms.clamp(16.0, 2000.0);
        c.scroll_pixels = c.scroll_pixels.clamp(1.0, 1000.0);
        Ok(c)
    }

    pub fn load(path: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(path) {
            Ok(t) => Config::parse(&t).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }
}

/// Set `key = value` in config text: replaces the first active line for `key`,
/// or appends one. Comments and other lines are kept as they are.
pub fn set_option(text: &str, key: &str, value: &str) -> String {
    let mut done = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            let k = strip_comment(l).split('=').next().unwrap_or("").trim();
            if !done && k == key && l.contains('=') {
                done = true;
                format!("{key} = {value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !done {
        out.push(format!("{key} = {value}"));
    }
    out.join("\n") + "\n"
}

/// `#` starts a comment unless inside quotes.
fn strip_comment(line: &str) -> &str {
    let mut q: Option<char> = None;
    for (i, c) in line.char_indices() {
        match (q, c) {
            (None, '#') => return &line[..i],
            (None, '"' | '\'') => q = Some(c),
            (Some(x), c) if c == x => q = None,
            _ => {}
        }
    }
    line
}

pub const TEMPLATE: &str = r#"# keyhop configuration
# Lines are `key = value`. `#` starts a comment.

# --- Keyboard / mouse sharing -------------------------------------------
# Address this machine listens on. Use a LAN IP instead of 0.0.0.0 to restrict it.
listen = 0.0.0.0:24801
# The other computer (host:port). Leave empty to only accept incoming connections.
peer =
# Which screen edge leads to the other computer: left | right | top | bottom | off
# (Edge switching works on macOS and Windows; on Linux use the hotkey.)
peer_edge = off
# Hotkey that moves control to / back from the other computer. `off` to disable.
switch_hotkey = ctrl+alt+f19

# --- File transfer --------------------------------------------------------
receive_dir = ~/Downloads/keyhop

# --- Smooth scrolling (for notched wheels like the MX Vertical) -----------
smooth_scroll = off
scroll_pixels = 48   # macOS only: pixels per wheel notch
scroll_ms = 160      # animation length

# --- Accent-menu lights (macOS, Keychron V1 Ultra 8K) ---------------------
# Holding a letter opens macOS's accent menu; this lights only the number
# keys that pick an accent, then restores your lighting. Never saved to the keyboard.
accent_lights = off
# Terminal hints: light the keys that can come next in the current word (zsh).
# Needs the zsh hook that `keyhop setup` adds to ~/.zshrc.
terminal_hints = off

# --- App-launch hotkeys ---------------------------------------------------
# Map spare keys to F13-F20 in Keychron Launcher / VIA, then bind them here.
# Run `keyhop keys` to see the name of any key you press.
# bind f13 = open -a Safari                      # macOS
# bind f14 = firefox                             # Linux
# bind f15 = cmd /c start "" "C:\Program Files\App\app.exe"   # Windows
# bind ctrl+shift+f16 = code
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_template_and_bindings() {
        let c = Config::parse(TEMPLATE).unwrap();
        assert_eq!(c.listen, "0.0.0.0:24801");
        assert!(c.peer.is_none());
        let c = Config::parse(
            "peer = 10.0.0.2:24801\npeer_edge = right\nbind ctrl+f13 = open -a \"Visual Studio Code\" # c\n",
        )
        .unwrap();
        assert_eq!(c.peer_edge, Some(Edge::Right));
        assert_eq!(c.bindings[0].argv, vec!["open", "-a", "Visual Studio Code"]);
        assert_eq!(c.bindings[0].hotkey.mods, keys::MOD_CTRL);
        assert!(Config::parse("nonsense = 1").is_err());
    }

    #[test]
    fn sets_options() {
        let t = set_option(TEMPLATE, "terminal_hints", "on");
        assert!(Config::parse(&t).unwrap().terminal_hints);
        assert_eq!(t.matches("terminal_hints = ").count(), 1);
        let t = set_option("# smooth_scroll = on\n", "smooth_scroll", "on");
        assert_eq!(t, "# smooth_scroll = on\nsmooth_scroll = on\n");
    }

    #[test]
    fn splits_args() {
        assert_eq!(split_args(r#"a "b c" 'd e' f"#).unwrap(), vec!["a", "b c", "d e", "f"]);
        assert_eq!(split_args(r#""C:\Program Files\x.exe""#).unwrap(), vec![r"C:\Program Files\x.exe"]);
        assert!(split_args("\"open").is_err());
    }
}
