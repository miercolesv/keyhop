//! `keyhop setup`: guided first-time setup. Installs the binary in a fixed
//! place, writes the config from a few questions, installs the zsh hint hook
//! and the login service. Safe to run again: it updates rather than duplicates.

use crate::{config, device};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The zsh hook, shipped inside the binary so setup needs no source checkout.
const HINTS_ZSH: &str = include_str!("../packaging/keyhop-hints.zsh");

fn ask(question: &str, default: bool) -> bool {
    print!("{question} [{}] ", if default { "Y/n" } else { "y/N" });
    let _ = std::io::stdout().flush();
    let mut a = String::new();
    if std::io::stdin().read_line(&mut a).is_err() {
        return default;
    }
    match a.trim().to_ascii_lowercase().as_str() {
        "" => default,
        s => s.starts_with('y'),
    }
}

/// Where setup keeps the binary, so services and permissions don't depend on
/// the build folder.
fn install_path() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(config::home);
        base.join("keyhop").join("keyhop.exe")
    } else {
        config::home().join(".local/bin/keyhop")
    }
}

/// Copy the running binary to `dst` (via a temporary file, so a running copy
/// is replaced atomically).
fn install_binary(dst: &Path) -> Result<(), String> {
    let src = std::env::current_exe().map_err(|e| e.to_string())?;
    if src.canonicalize().ok() == dst.canonicalize().ok() {
        return Ok(());
    }
    std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = dst.with_extension("new");
    std::fs::copy(&src, &tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, dst).map_err(|e| format!("{}: {e}", dst.display()))
}

/// Add `source <hook>` to ~/.zshrc (replacing an older keyhop hook line),
/// keeping a one-time backup.
fn add_zsh_hook(hook: &Path) -> Result<PathBuf, String> {
    let rc = config::home().join(".zshrc");
    let old = std::fs::read_to_string(&rc).unwrap_or_default();
    let line = format!("source \"{}\"", hook.display());
    let new = zshrc_with_hook(&old, &line);
    if new != old {
        let backup = config::home().join(".zshrc.keyhop-backup");
        if rc.exists() && !backup.exists() {
            std::fs::copy(&rc, &backup).map_err(|e| e.to_string())?;
        }
        std::fs::write(&rc, new).map_err(|e| e.to_string())?;
    }
    Ok(rc)
}

fn zshrc_with_hook(rc: &str, line: &str) -> String {
    if rc.lines().any(|l| l.trim() == line) {
        return rc.to_string();
    }
    let kept: Vec<&str> = rc.lines().filter(|l| !(l.contains("keyhop-hints.zsh") && l.trim_start().starts_with("source"))).collect();
    let mut out = kept.join("\n");
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.contains("# keyhop:") {
        out.push_str("\n# keyhop: light the keys that can come next while typing\n");
    }
    out.push_str(line);
    out.push('\n');
    out
}

pub fn run() -> Result<(), String> {
    println!("keyhop setup\n");

    let exe = install_path();
    install_binary(&exe)?;
    println!("binary     {}", exe.display());

    let path = config::config_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let existing = std::fs::read_to_string(&path).ok();
    let mut text = existing.clone().unwrap_or_else(|| config::TEMPLATE.to_string());
    // On a re-run, Enter keeps the current answer; first time, the recommended one.
    let cur = existing.as_deref().map(config::Config::parse).and_then(Result::ok);
    let default = |current: fn(&config::Config) -> bool, first: bool| cur.as_ref().map_or(first, current);
    crate::net::Identity::load_or_create().map_err(|e| format!("identity: {e}"))?;

    let set = |t: &mut String, k: &str, on: bool| *t = config::set_option(t, k, if on { "on" } else { "off" });
    set(&mut text, "smooth_scroll", ask("Smooth scrolling for notched mouse wheels?", default(|c| c.smooth_scroll, false)));

    let keyboard = device::supported_keyboard();
    let zsh = std::env::var("SHELL").is_ok_and(|s| s.ends_with("zsh"));
    let mut hints = false;
    match keyboard {
        Some(pid) => {
            println!("keyboard   Keychron 3434:{pid:04X} found");
            if cfg!(target_os = "macos") {
                set(&mut text, "accent_lights", ask("Light the number keys of the accent menu when you hold a letter?", default(|c| c.accent_lights, true)));
            }
            if cfg!(unix) && zsh {
                hints = ask(
                    "Light the keys that can come next while typing in the terminal (zsh)?",
                    default(|c| c.terminal_hints, true),
                );
                set(&mut text, "terminal_hints", hints);
            }
        }
        None => println!("keyboard   no supported Keychron keyboard found (keyboard features skipped)"),
    }
    std::fs::write(&path, &text).map_err(|e| e.to_string())?;
    println!("config     {}", path.display());

    if hints {
        let hook = config::config_dir().join("keyhop-hints.zsh");
        std::fs::write(&hook, HINTS_ZSH).map_err(|e| e.to_string())?;
        let rc = add_zsh_hook(&hook)?;
        println!("zsh hook   {} (sourced from {}; open a new terminal)", hook.display(), rc.display());
    }

    if cfg!(target_os = "linux") {
        println!(
            "\nLinux input access (once, needs sudo; then log out and back in):\n  sudo usermod -aG input $USER\n  sudo cp packaging/99-keyhop-uinput.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger\n  sudo modprobe uinput"
        );
    }

    if ask("Start keyhop automatically at login?", true) {
        println!("\n{}", install_service(&exe)?);
        if cfg!(target_os = "macos") {
            println!("\nmacOS will show permission prompts for keyhop. Switch it on under");
            println!("Accessibility and Input Monitoring; keyhop starts working within about 10 s.");
        }
    } else {
        println!("\nStart it yourself with: {} run", exe.display());
    }
    println!("\nDone. Settings: {}  (run `keyhop setup` again to change answers)", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_hook_is_added_once_and_replaces_old_lines() {
        let line = "source \"/h/.config/keyhop/keyhop-hints.zsh\"";
        let rc = zshrc_with_hook("export A=1\n", line);
        assert!(rc.ends_with(&format!("{line}\n")));
        assert_eq!(zshrc_with_hook(&rc, line), rc);
        let old = "export A=1\n# keyhop: x\nsource /repo/packaging/keyhop-hints.zsh\n";
        let rc = zshrc_with_hook(old, line);
        assert!(!rc.contains("/repo/"));
        assert_eq!(rc.matches("keyhop-hints.zsh").count(), 1);
    }
}

/// Start `exe run` at login: launchd agent (macOS), systemd user unit (Linux)
/// or the HKCU Run key (Windows).
pub fn install_service(exe: &Path) -> Result<String, String> {
    let exe = exe.to_string_lossy().to_string();
    #[cfg(target_os = "macos")]
    {
        let home = config::home();
        let dir = home.join("Library/LaunchAgents");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let plist = dir.join("dev.keyhop.plist");
        let log = home.join("Library/Logs/keyhop.log");
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>dev.keyhop</string>
  <key>ProgramArguments</key><array><string>{exe}</string><string>run</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
            log.display()
        );
        std::fs::write(&plist, body).map_err(|e| e.to_string())?;
        let _ = std::process::Command::new("launchctl").args(["unload", &plist.to_string_lossy()]).status();
        let ok = std::process::Command::new("launchctl")
            .args(["load", "-w", &plist.to_string_lossy()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return Ok(format!(
            "{} {}\nLog: {}\nmacOS asks to allow {exe} under Accessibility and Input Monitoring: switch both on.\nRemove with: launchctl unload -w {0}",
            plist.display(),
            if ok { "loaded" } else { "written (load it with launchctl)" },
            log.display()
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let dir = config::home().join(".config/systemd/user");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let unit = dir.join("keyhop.service");
        std::fs::write(
            &unit,
            format!(
                "[Unit]\nDescription=keyhop\nAfter=graphical-session.target\n\n[Service]\nExecStart={exe} run\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
            ),
        )
        .map_err(|e| e.to_string())?;
        let ok = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .and_then(|_| std::process::Command::new("systemctl").args(["--user", "enable", "--now", "keyhop"]).status())
            .map(|s| s.success())
            .unwrap_or(false);
        return Ok(format!(
            "{} {}\nLogs: journalctl --user -u keyhop -f\nRemove with: systemctl --user disable --now keyhop",
            unit.display(),
            if ok { "enabled and started" } else { "written (enable with systemctl --user enable --now keyhop)" }
        ));
    }
    #[cfg(windows)]
    {
        let ok = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "keyhop",
                "/t",
                "REG_SZ",
                "/d",
                &format!("\"{exe}\" run --background"),
                "/f",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return if ok {
            Ok(r"Added to HKCU\...\Run (starts at next login). Remove with: reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v keyhop /f".into())
        } else {
            Err("reg add failed".into())
        };
    }
    #[allow(unreachable_code)]
    Err(format!("unsupported platform ({exe})"))
}

