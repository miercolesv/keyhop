//! Terminal hints: the zsh hook (packaging/keyhop-hints.zsh) connects to a
//! local socket after every keystroke and writes one line with the characters
//! that may come next in the current word (an empty line clears the hint).

use crate::{config, keys, lights::Msg};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub fn socket_path() -> std::path::PathBuf {
    config::config_dir().join("hints.sock")
}

/// Keys for the characters in one hint line, without duplicates.
pub fn parse(line: &str) -> Vec<u16> {
    let mut out = Vec::new();
    for h in line.trim_end_matches(['\r', '\n']).chars().filter_map(keys::from_char) {
        if !out.contains(&h) {
            out.push(h);
        }
    }
    out
}

pub fn serve(tx: Sender<Msg>) -> Result<(), String> {
    let path = socket_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path); // stale socket from a previous run
    let listener = UnixListener::bind(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    crate::log!("terminal hints on {}", path.display());
    std::thread::Builder::new()
        .name("terminal-hints".into())
        .spawn(move || {
            for mut s in listener.incoming().flatten() {
                let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
                let mut buf = String::new();
                if (&mut s).take(4096).read_to_string(&mut buf).is_ok() {
                    let line = buf.lines().last().unwrap_or("");
                    if tx.send(Msg::Hint(parse(line))).is_err() {
                        return;
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_hint_lines() {
        assert_eq!(super::parse("dDe_-\n"), vec![0x07, 0x08, 0x2D]);
        assert!(super::parse("\n").is_empty());
    }
}
