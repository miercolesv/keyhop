//! Encrypted, mutually authenticated channel (Noise XX over TCP), identity and
//! trust store, pairing, and the session listener/dialer.

use crate::config::{config_dir, Config};
use crate::engine::{Engine, Link};
use crate::proto::{self, Msg, MAX_PLAIN};
use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const PROLOGUE: &[u8] = b"keyhop/1";
pub type Key = [u8; 32];

fn other(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

// ---------------------------------------------------------------- identity

pub struct Identity {
    pub private: Key,
    pub public: Key,
}

impl Identity {
    pub fn path() -> PathBuf {
        config_dir().join("identity.key")
    }

    pub fn load_or_create() -> io::Result<Identity> {
        let p = Self::path();
        if let Ok(b) = std::fs::read(&p) {
            if b.len() == 64 {
                return Ok(Identity { private: b[..32].try_into().unwrap(), public: b[32..].try_into().unwrap() });
            }
            return Err(other(format!("{} is corrupt", p.display())));
        }
        let kp = snow::Builder::new(PARAMS.parse().unwrap()).generate_keypair().map_err(other)?;
        let id = Identity {
            private: kp.private.as_slice().try_into().map_err(other)?,
            public: kp.public.as_slice().try_into().map_err(other)?,
        };
        std::fs::create_dir_all(config_dir())?;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&p)?;
        f.write_all(&id.private)?;
        f.write_all(&id.public)?;
        Ok(id)
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Key> {
    if s.len() != 64 {
        return None;
    }
    let mut k = [0u8; 32];
    for (i, c) in k.iter_mut().enumerate() {
        *c = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(k)
}

/// Short human-readable fingerprint.
pub fn fingerprint(k: &Key) -> String {
    let h = hex(&k[..10]);
    h.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("-")
}

// ---------------------------------------------------------------- trust store

#[derive(Default, Clone)]
pub struct Trusted(pub Vec<(Key, String)>);

impl Trusted {
    pub fn path() -> PathBuf {
        config_dir().join("trusted")
    }
    pub fn load() -> Trusted {
        let t = std::fs::read_to_string(Self::path()).unwrap_or_default();
        Trusted(
            t.lines()
                .filter_map(|l| {
                    let (k, n) = l.trim().split_once(' ').unwrap_or((l.trim(), ""));
                    Some((unhex(k)?, n.trim().to_string()))
                })
                .collect(),
        )
    }
    pub fn save(&self) -> io::Result<()> {
        std::fs::create_dir_all(config_dir())?;
        let s: String = self.0.iter().map(|(k, n)| format!("{} {}\n", hex(k), n)).collect();
        std::fs::write(Self::path(), s)
    }
    pub fn get(&self, k: &Key) -> Option<&str> {
        self.0.iter().find(|(x, _)| x == k).map(|(_, n)| n.as_str())
    }
    pub fn add(&mut self, k: Key, name: String) {
        self.0.retain(|(x, _)| x != &k);
        self.0.push((k, name));
    }
}

pub fn hostname() -> String {
    if let Ok(n) = std::env::var("COMPUTERNAME") {
        return n;
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

// ---------------------------------------------------------------- channel

pub struct Channel {
    stream: TcpStream,
    noise: Mutex<snow::TransportState>,
    write_lock: Mutex<()>,
    pub remote: Key,
    pub initiator: bool,
    pub hash: [u8; 32],
}

fn read_frame(mut s: &TcpStream, buf: &mut Vec<u8>) -> io::Result<()> {
    let mut len = [0u8; 2];
    s.read_exact(&mut len)?;
    buf.resize(u16::from_be_bytes(len) as usize, 0);
    s.read_exact(buf)
}

fn write_frame(mut s: &TcpStream, data: &[u8]) -> io::Result<()> {
    let mut out = Vec::with_capacity(data.len() + 2);
    out.extend_from_slice(&(data.len() as u16).to_be_bytes());
    out.extend_from_slice(data);
    s.write_all(&out)
}

impl Channel {
    fn handshake(stream: TcpStream, id: &Identity, initiator: bool) -> io::Result<Channel> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let b = snow::Builder::new(PARAMS.parse().unwrap())
            .local_private_key(&id.private)
            .and_then(|b| b.prologue(PROLOGUE))
            .map_err(other)?;
        let mut hs = if initiator { b.build_initiator() } else { b.build_responder() }.map_err(other)?;
        let (mut msg, mut payload, mut frame) = (vec![0u8; 1024], vec![0u8; 1024], Vec::new());
        for step in 0..3 {
            let my_turn = (step % 2 == 0) == initiator;
            if my_turn {
                let n = hs.write_message(&[], &mut msg).map_err(other)?;
                write_frame(&stream, &msg[..n])?;
            } else {
                read_frame(&stream, &mut frame)?;
                hs.read_message(&frame, &mut payload).map_err(|_| other("handshake failed"))?;
            }
        }
        let remote: Key = hs.get_remote_static().ok_or_else(|| other("no remote key"))?.try_into().map_err(other)?;
        let hash: [u8; 32] = hs.get_handshake_hash()[..32].try_into().unwrap();
        let noise = hs.into_transport_mode().map_err(other)?;
        Ok(Channel { stream, noise: Mutex::new(noise), write_lock: Mutex::new(()), remote, initiator, hash })
    }

    pub fn connect(addr: &str, id: &Identity) -> io::Result<Channel> {
        let sa = addr.to_socket_addrs()?.next().ok_or_else(|| other("cannot resolve peer"))?;
        let s = TcpStream::connect_timeout(&sa, Duration::from_secs(5))?;
        Self::handshake(s, id, true)
    }

    pub fn accept(s: TcpStream, id: &Identity) -> io::Result<Channel> {
        Self::handshake(s, id, false)
    }

    pub fn send(&self, m: &Msg) -> io::Result<()> {
        let mut plain = Vec::with_capacity(32);
        m.encode(&mut plain);
        debug_assert!(plain.len() <= MAX_PLAIN);
        let _w = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut ct = vec![0u8; plain.len() + 16];
        let n = self.noise.lock().unwrap_or_else(|p| p.into_inner()).write_message(&plain, &mut ct).map_err(other)?;
        write_frame(&self.stream, &ct[..n])
    }

    pub fn recv(&self, frame: &mut Vec<u8>, plain: &mut Vec<u8>) -> io::Result<Msg> {
        read_frame(&self.stream, frame)?;
        plain.resize(frame.len(), 0);
        let n = self
            .noise
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .read_message(frame, plain)
            .map_err(|_| other("decrypt failed"))?;
        Msg::decode(&plain[..n]).ok_or_else(|| other("bad message"))
    }

    pub fn set_timeout(&self, d: Option<Duration>) {
        let _ = self.stream.set_read_timeout(d);
    }

    pub fn shutdown(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }

    /// 6-digit short authentication string both sides must see identically.
    pub fn sas(&self) -> String {
        let v = u32::from_le_bytes(self.hash[..4].try_into().unwrap()) % 1_000_000;
        format!("{:03} {:03}", v / 1000, v % 1000)
    }
}

// ---------------------------------------------------------------- pairing

pub fn pair(cfg: &Config, id: &Identity, target: Option<&str>) -> Result<(), String> {
    let ch = match target {
        Some(addr) => {
            println!("Connecting to {addr} ...");
            Channel::connect(addr, id).map_err(|e| e.to_string())?
        }
        None => {
            let l = TcpListener::bind(&cfg.listen).map_err(|e| format!("bind {}: {e} (is the service running? stop it while pairing)", cfg.listen))?;
            println!("Waiting for a pairing request on {} ...", cfg.listen);
            let (s, a) = l.accept().map_err(|e| e.to_string())?;
            println!("Connection from {a}");
            Channel::accept(s, id).map_err(|e| e.to_string())?
        }
    };
    ch.set_timeout(Some(Duration::from_secs(300)));
    let (mut f, mut p) = (Vec::new(), Vec::new());
    let me = hostname();
    if ch.initiator {
        ch.send(&Msg::Hello { kind: proto::KIND_PAIR, name: me.clone() }).map_err(|e| e.to_string())?;
    } else {
        match ch.recv(&mut f, &mut p) {
            Ok(Msg::Hello { kind: proto::KIND_PAIR, .. }) => {}
            _ => return Err("peer did not request pairing".into()),
        }
    }
    println!("\nPeer key:  {}", fingerprint(&ch.remote));
    println!("Your key:  {}", fingerprint(&id.public));
    println!("\n   Pairing code:  {}\n", ch.sas());
    print!("Does the other computer show the same code? [y/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    let yes = line.trim().eq_ignore_ascii_case("y") || line.trim().eq_ignore_ascii_case("yes");
    ch.send(&Msg::Confirm { yes, name: me }).map_err(|e| e.to_string())?;
    if !yes {
        return Err("pairing cancelled".into());
    }
    println!("Waiting for the other side to confirm ...");
    match ch.recv(&mut f, &mut p) {
        Ok(Msg::Confirm { yes: true, name }) => {
            let mut t = Trusted::load();
            let name = name.chars().filter(|c| !c.is_control()).take(64).collect::<String>().replace(' ', "_");
            t.add(ch.remote, name.clone());
            t.save().map_err(|e| e.to_string())?;
            println!("Paired with {name}. Saved to {}", Trusted::path().display());
            Ok(())
        }
        _ => Err("the other side rejected pairing".into()),
    }
}

// ---------------------------------------------------------------- session service

/// Spawns the listener (and dialer, if a peer is configured). Non-blocking.
pub fn serve(engine: Arc<Engine>, id: Arc<Identity>, trusted: Trusted) -> Result<(), String> {
    let cfg = engine.config().clone();
    let listener = TcpListener::bind(&cfg.listen).map_err(|e| format!("bind {}: {e}", cfg.listen))?;
    crate::log!("listening on {} as {}", cfg.listen, fingerprint(&id.public));
    let trusted = Arc::new(trusted);
    {
        let (engine, id, trusted) = (engine.clone(), id.clone(), trusted.clone());
        std::thread::Builder::new()
            .name("listener".into())
            .spawn(move || {
                for s in listener.incoming().flatten() {
                    let (engine, id, trusted) = (engine.clone(), id.clone(), trusted.clone());
                    std::thread::spawn(move || {
                        let peer = s.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                        match Channel::accept(s, &id) {
                            Ok(ch) => handle(ch, engine, &id, &trusted),
                            Err(e) => crate::log!("handshake with {peer} failed: {e}"),
                        }
                    });
                }
            })
            .map_err(|e| e.to_string())?;
    }
    if let Some(addr) = cfg.peer.clone() {
        std::thread::Builder::new()
            .name("dialer".into())
            .spawn(move || loop {
                if !engine.has_link() {
                    if let Ok(ch) = Channel::connect(&addr, &id) { handle(ch, engine.clone(), &id, &trusted) }
                }
                std::thread::sleep(Duration::from_secs(3));
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn handle(ch: Channel, engine: Arc<Engine>, id: &Identity, trusted: &Trusted) {
    let Some(name) = trusted.get(&ch.remote).map(str::to_string) else {
        crate::log!("rejected untrusted peer {} (run `keyhop pair`)", fingerprint(&ch.remote));
        ch.shutdown();
        return;
    };
    let (mut f, mut p) = (Vec::new(), Vec::new());
    let kind = if ch.initiator {
        if ch.send(&Msg::Hello { kind: proto::KIND_SESSION, name: hostname() }).is_err() {
            return;
        }
        proto::KIND_SESSION
    } else {
        match ch.recv(&mut f, &mut p) {
            Ok(Msg::Hello { kind, .. }) => kind,
            _ => return ch.shutdown(),
        }
    };
    match kind {
        proto::KIND_SESSION => session(ch, engine, id, &name),
        proto::KIND_FILE => crate::files::receive(ch, engine.config(), &name),
        _ => ch.shutdown(),
    }
}

fn session(ch: Channel, engine: Arc<Engine>, id: &Identity, name: &str) {
    let ch = Arc::new(ch);
    // Deterministic tie-break when both sides dial at once: keep the session
    // initiated by the side with the smaller public key.
    let initiator_key = if ch.initiator { id.public } else { ch.remote };
    let preferred = initiator_key == id.public.min(ch.remote);
    let (tx, rx) = channel::<Msg>();
    let link_id = engine.new_link_id();
    if !engine.attach(Link { id: link_id, tx, chan: ch.clone(), preferred }) {
        ch.shutdown();
        return;
    }
    crate::log!("session with {name}");
    let w = ch.clone();
    std::thread::spawn(move || loop {
        let m = match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(m) => m,
            Err(RecvTimeoutError::Timeout) => Msg::Ping,
            Err(RecvTimeoutError::Disconnected) => return w.shutdown(),
        };
        // Coalesce queued pointer motion so slow links don't build latency.
        let m = match m {
            Msg::Motion { dx, dy } => {
                let (mut x, mut y) = (dx as i32, dy as i32);
                let mut next = None;
                while let Ok(n) = rx.try_recv() {
                    match n {
                        Msg::Motion { dx, dy } if (x + dx as i32).abs() < 32000 && (y + dy as i32).abs() < 32000 => {
                            x += dx as i32;
                            y += dy as i32;
                        }
                        other => {
                            next = Some(other);
                            break;
                        }
                    }
                }
                if w.send(&Msg::Motion { dx: x as i16, dy: y as i16 }).is_err() {
                    return w.shutdown();
                }
                match next {
                    Some(n) => n,
                    None => continue,
                }
            }
            m => m,
        };
        if w.send(&m).is_err() {
            return w.shutdown();
        }
    });
    ch.set_timeout(Some(Duration::from_secs(8)));
    let (mut f, mut p) = (Vec::with_capacity(64), Vec::with_capacity(64));
    while let Ok(m) = ch.recv(&mut f, &mut p) {
        if !matches!(m, Msg::Ping) {
            engine.on_remote(m);
        }
    }
    engine.detach(link_id);
    ch.shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident() -> Identity {
        let kp = snow::Builder::new(PARAMS.parse().unwrap()).generate_keypair().unwrap();
        Identity { private: kp.private.try_into().unwrap(), public: kp.public.try_into().unwrap() }
    }

    #[test]
    fn handshake_and_transport() {
        let (a, b) = (ident(), ident());
        let (apub, bpub) = (a.public, b.public);
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let ch = Channel::accept(s, &b).unwrap();
            let (mut f, mut p) = (Vec::new(), Vec::new());
            let m = ch.recv(&mut f, &mut p).unwrap();
            ch.send(&Msg::FileData(vec![7; proto::FILE_CHUNK])).unwrap();
            (ch.remote, ch.sas(), m)
        });
        let ch = Channel::connect(&addr, &a).unwrap();
        assert_eq!(ch.remote, bpub);
        ch.send(&Msg::Key { hid: 4, state: 1 }).unwrap();
        let (mut f, mut p) = (Vec::new(), Vec::new());
        assert_eq!(ch.recv(&mut f, &mut p).unwrap(), Msg::FileData(vec![7; proto::FILE_CHUNK]));
        // Ciphertext on the wire must not contain the plaintext.
        assert!(!f.windows(64).any(|w| w.iter().all(|&x| x == 7)));
        let (remote, sas, m) = t.join().unwrap();
        assert_eq!(remote, apub);
        assert_eq!(sas, ch.sas());
        assert_eq!(m, Msg::Key { hid: 4, state: 1 });
    }

    #[test]
    fn hex_roundtrip() {
        let k = [0xABu8; 32];
        assert_eq!(unhex(&hex(&k)), Some(k));
        assert_eq!(fingerprint(&k).len(), 24);
    }
}
