# Contributing

## Build and test

```sh
cargo build --release
cargo test
```

Every change must keep all three platforms compiling. `cargo clippy` needs no
linker or SDK, so you can check every target from any machine:

```sh
rustup target add aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
for t in aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-gnu; do
  cargo clippy --target "$t"
done
```

Clippy must report no warnings.

With a supported keyboard attached, one extra test checks the lighting code on
the real hardware (it lights keys 1–3 for 3 seconds in keyboard RAM, then checks
that everything was restored):

```sh
cargo test -- --ignored lights_live
```

## Documentation site

The documentation in `docs/` is a VitePress site, published to GitHub Pages by
`.github/workflows/docs.yml` on every push to `main`. To preview it locally:

```sh
cd docs
npm install
npm run dev          # live preview; `npm run build` for the static site
```

In the repository settings, set **Pages → Source** to **GitHub Actions** once.

## Releases

1. Update the version in `Cargo.toml` and add a section to `CHANGELOG.md`.
2. Push a tag: `git tag v0.1.0 && git push origin v0.1.0`.

`.github/workflows/release.yml` then builds every platform on GitHub's runners
(macOS universal, Linux x86_64/aarch64 static, Windows x86_64), with build
paths remapped, stripped binaries and archives without owner names, and
publishes a release with `SHA256SUMS` and the changelog section as notes.

## Ground rules

* keyhop stays a **single small native binary**: no GUI frameworks, web views,
  Electron, Python or async runtime in the app. Add a dependency only when it
  replaces a lot of risky code.
* The platform API in `src/platform/mod.rs` is shared: every backend keeps the
  same function names and signatures.
* Keys travel as USB HID usages; consumer keys as `0x1000 | usage`. New keys go
  in `src/keys.rs` (`TABLE` / `CONSUMER_TABLE`), and the table tests must pass.
* **No vendor code, assets or branding.** Firmware may be analysed only for
  facts (USB IDs, report layouts, command numbers), never copied.
* Keyboard writes: only settings the manufacturer's app can also change, and
  only on firmware that has been verified (`VERIFIED_PIDS` in `src/device.rs`).
  Never send factory/test commands, never flash firmware.
* Hint lighting must stay RAM-only (never send the keyboard's save commands).

## Adding a keyboard

1. Collect facts from the firmware and the live device: USB IDs, the raw HID
   command set, matrix size, LED map. `tools/fw-analysis/` has the helper
   scripts used for the first keyboard.
2. Write them down in `docs/hardware/<keyboard>.md`.
3. Check that every command keyhop sends behaves the same, then add the product
   ID to `VERIFIED_PIDS`.

## Project layout

```
src/main.rs            CLI
src/setup.rs           guided setup and login service
src/config.rs          config parser (no serde)
src/keys.rs            HID <-> macOS / evdev / Windows key tables
src/engine.rs          hotkeys, switching, forwarding, injection state machine
src/proto.rs           wire messages
src/net.rs             Noise channel, identity, trust store, pairing, sessions
src/files.rs           file transfer
src/smooth.rs          scroll smoother
src/device.rs          keyboard settings and RAM-only lighting over vendor raw HID
src/lights.rs          the thread that owns hint lighting
src/accents.rs         accent-menu hold detection (macOS)
src/hints.rs           Unix socket the zsh hook writes hint lines to
src/platform/          macos.rs (CGEventTap), linux.rs (evdev/uinput), windows.rs (LL hooks),
                       hid_*.rs (raw HID: IOHIDManager, hidraw, HID class driver)
packaging/             udev rule, zsh hook (embedded in the binary by setup)
docs/                  documentation site (VitePress) and hardware notes
tools/fw-analysis/     development scripts for firmware analysis (not part of the app)
```
