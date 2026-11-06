//! File transfer over the encrypted channel. Each `keyhop send` opens its own
//! connection, so transfers never block keyboard/mouse traffic.

use crate::config::Config;
use crate::net::{fingerprint, hostname, Channel, Identity, Trusted};
use crate::proto::{Msg, FILE_CHUNK, KIND_FILE};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Keep only a plain file name: no directories, traversal or control characters.
pub fn sanitize(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let s: String = base
        .chars()
        .map(|c| if c.is_control() || matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .take(200)
        .collect();
    let s = s.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string();
    if s.is_empty() {
        "file".into()
    } else {
        s
    }
}

fn unique(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap()
}

pub fn receive(ch: Channel, cfg: &Config, from: &str) {
    ch.set_timeout(Some(Duration::from_secs(30)));
    let (mut f, mut p) = (Vec::with_capacity(FILE_CHUNK + 64), Vec::with_capacity(FILE_CHUNK + 64));
    let mut cur: Option<(BufWriter<File>, PathBuf, String, u64, u64)> = None;
    while let Ok(m) = ch.recv(&mut f, &mut p) {
        match m {
            Msg::FileStart { size, name } => {
                let name = sanitize(&name);
                if std::fs::create_dir_all(&cfg.receive_dir).is_err() {
                    let _ = ch.send(&Msg::FileAck { ok: false });
                    break;
                }
                let tmp = cfg.receive_dir.join(format!(".{name}.keyhop-part"));
                match File::create(&tmp) {
                    Ok(file) => cur = Some((BufWriter::with_capacity(1 << 20, file), tmp, name, size, 0)),
                    Err(e) => {
                        crate::log!("cannot write {}: {e}", tmp.display());
                        let _ = ch.send(&Msg::FileAck { ok: false });
                        break;
                    }
                }
            }
            Msg::FileData(d) => {
                let Some((w, _, _, size, got)) = cur.as_mut() else { break };
                *got += d.len() as u64;
                if *got > *size || w.write_all(&d).is_err() {
                    break;
                }
            }
            Msg::FileEnd => {
                let Some((w, tmp, name, size, got)) = cur.take() else { break };
                let ok = got == size && w.into_inner().map(|f| f.sync_all().is_ok()).unwrap_or(false);
                let ok = ok && {
                    let dst = unique(&cfg.receive_dir, &name);
                    let r = std::fs::rename(&tmp, &dst).is_ok();
                    if r {
                        crate::log!("received {} ({} bytes) from {from}", dst.display(), size);
                    }
                    r
                };
                if !ok {
                    let _ = std::fs::remove_file(&tmp);
                }
                if ch.send(&Msg::FileAck { ok }).is_err() {
                    break;
                }
            }
            _ => break,
        }
    }
    if let Some((_, tmp, ..)) = cur {
        let _ = std::fs::remove_file(tmp);
    }
    ch.shutdown();
}

pub fn send(cfg: &Config, id: &Identity, paths: &[String]) -> Result<(), String> {
    let addr = cfg.peer.as_deref().ok_or("set `peer = host:port` in the config first")?;
    let trusted = Trusted::load();
    let ch = Channel::connect(addr, id).map_err(|e| format!("connect {addr}: {e}"))?;
    let name = trusted
        .get(&ch.remote)
        .ok_or_else(|| format!("peer {} is not paired (run `keyhop pair`)", fingerprint(&ch.remote)))?
        .to_string();
    ch.set_timeout(Some(Duration::from_secs(60)));
    ch.send(&Msg::Hello { kind: KIND_FILE, name: hostname() }).map_err(|e| e.to_string())?;
    send_over(&ch, &name, paths)?;
    ch.shutdown();
    Ok(())
}

/// Stream files over an established channel (after the Hello).
pub fn send_over(ch: &Channel, name: &str, paths: &[String]) -> Result<(), String> {
    let (mut f, mut p) = (Vec::new(), Vec::new());
    let mut buf = vec![0u8; FILE_CHUNK];
    for path in paths {
        let mut file = File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        let fname = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
        ch.send(&Msg::FileStart { size, name: fname.clone() }).map_err(|e| e.to_string())?;
        let t0 = Instant::now();
        let (mut sent, mut last_pct) = (0u64, u64::MAX);
        loop {
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            ch.send(&Msg::FileData(buf[..n].to_vec())).map_err(|e| e.to_string())?;
            sent += n as u64;
            let pct = (sent * 100).checked_div(size).unwrap_or(100);
            if pct != last_pct {
                eprint!("\r{fname} -> {name}: {pct:3}%");
                last_pct = pct;
            }
        }
        if sent != size {
            return Err(format!("{path}: file changed while sending"));
        }
        ch.send(&Msg::FileEnd).map_err(|e| e.to_string())?;
        match ch.recv(&mut f, &mut p) {
            Ok(Msg::FileAck { ok: true }) => {
                let secs = t0.elapsed().as_secs_f64().max(1e-3);
                eprintln!("\r{fname} -> {name}: done, {:.1} MB/s", size as f64 / secs / 1e6);
            }
            _ => return Err(format!("{fname}: the receiver could not save the file")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize("../../etc/passwd"), "passwd");
        assert_eq!(sanitize("..\\..\\x.exe"), "x.exe");
        assert_eq!(sanitize("a:b?.txt"), "a_b_.txt");
        assert_eq!(sanitize(".."), "file");
        assert_eq!(sanitize("  .hidden "), "hidden");
        assert_eq!(sanitize("photo 1.jpg"), "photo 1.jpg");
    }

    #[test]
    fn transfers_files_end_to_end() {
        use crate::net::{Channel, Identity};
        use std::net::TcpListener;
        let mk = || {
            let kp = snow::Builder::new("Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap()).generate_keypair().unwrap();
            Identity { private: kp.private.try_into().unwrap(), public: kp.public.try_into().unwrap() }
        };
        let (a, b) = (mk(), mk());
        let root = std::env::temp_dir().join(format!("keyhop-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        let big: Vec<u8> = (0..3_000_000u32).map(|i| (i * 7 + i / 13) as u8).collect();
        std::fs::write(root.join("src/big.bin"), &big).unwrap();
        std::fs::write(root.join("src/empty.txt"), b"").unwrap();
        std::fs::write(root.join("src/dup.txt"), b"one").unwrap();
        let mut cfg = Config::default();
        cfg.receive_dir = root.join("in");
        std::fs::create_dir_all(&cfg.receive_dir).unwrap();
        std::fs::write(cfg.receive_dir.join("dup.txt"), b"existing").unwrap();
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let rcfg = cfg.clone();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let ch = Channel::accept(s, &b).unwrap();
            let (mut f, mut p) = (Vec::new(), Vec::new());
            assert!(matches!(ch.recv(&mut f, &mut p), Ok(Msg::Hello { kind: KIND_FILE, .. })));
            receive(ch, &rcfg, "test");
        });
        let ch = Channel::connect(&addr, &a).unwrap();
        ch.send(&Msg::Hello { kind: KIND_FILE, name: "t".into() }).unwrap();
        let paths: Vec<String> = ["big.bin", "empty.txt", "dup.txt"]
            .iter()
            .map(|n| root.join("src").join(n).to_string_lossy().into_owned())
            .collect();
        send_over(&ch, "test", &paths).unwrap();
        ch.shutdown();
        t.join().unwrap();
        assert_eq!(std::fs::read(cfg.receive_dir.join("big.bin")).unwrap(), big);
        assert_eq!(std::fs::read(cfg.receive_dir.join("empty.txt")).unwrap(), b"");
        assert_eq!(std::fs::read(cfg.receive_dir.join("dup.txt")).unwrap(), b"existing");
        assert_eq!(std::fs::read(cfg.receive_dir.join("dup (1).txt")).unwrap(), b"one");
        let leftovers = std::fs::read_dir(&cfg.receive_dir).unwrap().filter(|e| {
            e.as_ref().unwrap().file_name().to_string_lossy().ends_with("keyhop-part")
        });
        assert_eq!(leftovers.count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
