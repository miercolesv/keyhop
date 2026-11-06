# keyhop

A lean, independent replacement for the companion-app features of keyboard
"assistant" utilities: one small native binary for **macOS, Linux and Windows**,
configured with a text file and a handful of commands.

No web view, no Electron, no Python, no tray UI, no telemetry, no auto-updater,
and no outgoing connections except to the computer you pair with.

## Features

| Feature | macOS | Linux (X11 + Wayland) | Windows |
|---|---|---|---|
| App-launch hotkeys | ✅ | ✅ | ✅ |
| Smooth scrolling for notched mouse wheels | ✅ | ✅ | ✅ |
| Encrypted keyboard/mouse sharing between two computers | ✅ edge + hotkey | ✅ hotkey | ✅ edge + hotkey |
| Encrypted file transfer | ✅ | ✅ | ✅ |
| Keyboard settings: polling rate, debounce, sleep, snap-click, keys, lighting ¹ | ✅ | ✅ | ✅ ² |
| Accent-menu lights: holding a letter lights the number keys that pick an accent ¹ | ✅ | — | — |
| Terminal hints: lights the keys that can come next, like Tab would complete ¹ | ✅ | ✅ ² | — |

¹ Needs a supported keyboard (see below). ² Builds and type-checks; not yet tested on hardware.

### Supported keyboards

The keyboard features talk to the keyboard's vendor configuration interface.
They are enabled only for firmware that has been analysed and verified:

| Keyboard | USB ID | Notes |
|---|---|---|
| Keychron V1 Ultra 8K (ANSI) | `3434:0C14` | [hardware notes](docs/hardware/keychron-v1-ultra-8k.md) |

Everything else (hotkeys, scrolling, sharing, file transfer) works with any
keyboard and mouse.

**Documentation:** <https://miercolesv.github.io/keyhop/>

## Quick start

Download the archive for your platform from the
[releases page](https://github.com/miercolesv/keyhop/releases), unpack it and run:

```sh
./keyhop setup                   # answer a few questions, then you're done
```

Or build from source (needs a Rust toolchain):

```sh
cargo build --release
./target/release/keyhop setup
```

`keyhop setup` installs the binary to `~/.local/bin/keyhop`, writes the config,
offers the keyboard features if a supported keyboard is connected, adds the
terminal hook to `~/.zshrc` (with a backup) and starts keyhop at login. On macOS
it then shows the system prompts for **Accessibility** and **Input Monitoring**:
switch keyhop on in both and it starts working within about 10 seconds.

Run `keyhop setup` again any time to change your answers.

## Documentation

The full guide is at **<https://miercolesv.github.io/keyhop/>** (sources in
[`docs/`](docs/)):

* [Getting started](docs/guide/getting-started.md) · [Installation](docs/guide/installation.md) · [Troubleshooting](docs/guide/troubleshooting.md)
* Features: [hotkeys](docs/features/hotkeys.md), [smooth scrolling](docs/features/smooth-scrolling.md), [sharing and file transfer](docs/features/sharing.md), [keyboard settings](docs/features/keyboard-settings.md), [accent lights](docs/features/accent-lights.md), [terminal hints](docs/features/terminal-hints.md)
* Reference: [configuration](docs/reference/configuration.md), [commands](docs/reference/commands.md), [key names](docs/reference/key-names.md), [security model](docs/reference/security.md)
* [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

## Commands

```
keyhop setup                    guided first-time setup (start here)
keyhop run [--background]       run the service (foreground by default)
keyhop keys                     print the name of each key/button you press
keyhop pair --listen | <host:port>   pair with another computer
keyhop send <file>...           send files to the paired computer
keyhop device ...               keyboard settings (run `keyhop device` to see them)
keyhop install-service          start keyhop automatically at login
```

Run `keyhop` without arguments for the complete list.

## Limitations

* Linux has no edge switching (Wayland doesn't expose the global cursor
  position); use the hotkey.
* No clipboard sync, no screen mirroring, one paired computer at a time.
* No GUI: configuration is a commented text file.
* On the V1 Ultra 8K, lit keys can't be dimmed or switched off one by one (the
  firmware uses one brightness for all keys), so hints contrast by colour.

## License

MIT. See [LICENSE](LICENSE).

keyhop is an independent project. It is not affiliated with or endorsed by any
keyboard manufacturer, and it contains no vendor firmware or software. Product
names are used only to say which hardware is supported.
