# Installation

The [Getting started](/guide/getting-started) page covers the short version.
This page has the details.

## Requirements

* macOS 11 or newer, a Linux desktop (X11 or Wayland), or Windows 10/11.
* To build from source: a Rust toolchain (install it with rustup). No other
  build tools are needed.

## Pre-built binaries

Each [release](https://github.com/miercolesv/keyhop/releases) has a binary per
platform plus a `SHA256SUMS` file. Check the download, then run `setup` from it:

```sh
shasum -a 256 -c SHA256SUMS --ignore-missing   # Linux: sha256sum -c …
chmod +x keyhop && ./keyhop setup
```

## Guided setup (recommended)

```sh
cargo build --release
./target/release/keyhop setup
```

`keyhop setup`:

1. Copies the binary to a fixed place, `~/.local/bin/keyhop`
   (`%LOCALAPPDATA%\keyhop\keyhop.exe` on Windows), so the service and the
   permissions don't depend on the build folder.
2. Asks a few yes/no questions and writes `~/.config/keyhop/keyhop.conf`
   (`%APPDATA%\keyhop\keyhop.conf` on Windows). Keyboard features are offered
   only when a supported keyboard is connected, and terminal hints only when
   your shell is zsh. An existing config is updated, never replaced; on a second
   run, Enter keeps your current answer.
3. For terminal hints, installs the zsh hook into the config folder and adds one
   `source` line to the end of `~/.zshrc`. The first change makes a backup at
   `~/.zshrc.keyhop-backup`; the line is never added twice.
4. Optionally installs the login service (see below).
5. On Linux, prints the commands that give your user access to input devices.

It is safe to run again whenever you want to change something.

## Permissions

**macOS.** keyhop needs **Accessibility** and **Input Monitoring**. When the
service starts without them, macOS shows its own prompts for the keyhop
binary: switch it on in both lists (System Settings → Privacy & Security). The
service restarts by itself and works within about 10 seconds. When you run
`keyhop run` from a terminal instead, macOS asks for the terminal app.

macOS ties the permission to the exact binary. After installing a new build,
you may have to switch keyhop off and on again in both lists.

If a password field has focus, macOS Secure Input hides keystrokes from every
app like keyhop. That is expected.

**Linux.** Once, then log out and back in:

```sh
sudo usermod -aG input "$USER"
sudo cp packaging/99-keyhop-uinput.rules /etc/udev/rules.d/
sudo modprobe uinput && echo uinput | sudo tee /etc/modules-load.d/uinput.conf
sudo udevadm control --reload && sudo udevadm trigger
```

The rule also gives access to the raw HID interface of supported keyboards,
which `keyhop device` and the keyboard lighting use.

**Windows.** Nothing to set up. keyhop can't control apps running as
administrator unless keyhop itself runs as administrator.

## Login service

`keyhop install-service` (also offered by `keyhop setup`) starts `keyhop run`
at login:

| OS | How | Log |
|---|---|---|
| macOS | launchd agent `~/Library/LaunchAgents/dev.keyhop.plist` | `~/Library/Logs/keyhop.log` |
| Linux | systemd user unit `~/.config/systemd/user/keyhop.service` | `journalctl --user -u keyhop -f` |
| Windows | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | — |

## Updating

Download the new release and run `./keyhop setup` from it, or rebuild:

```sh
git pull
cargo build --release
./target/release/keyhop setup     # copies the new binary and restarts the service
```

On macOS, re-enable keyhop under Accessibility and Input Monitoring if the
prompts appear again.

## Uninstalling

macOS:

```sh
launchctl unload -w ~/Library/LaunchAgents/dev.keyhop.plist
rm ~/Library/LaunchAgents/dev.keyhop.plist ~/.local/bin/keyhop
```

Linux:

```sh
systemctl --user disable --now keyhop
rm ~/.config/systemd/user/keyhop.service ~/.local/bin/keyhop
sudo rm /etc/udev/rules.d/99-keyhop-uinput.rules
```

Windows:

```bat
reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v keyhop /f
rmdir /s %LOCALAPPDATA%\keyhop
```

Then remove the `# keyhop` lines at the end of `~/.zshrc` (or restore
`~/.zshrc.keyhop-backup`), and delete the config folder if you don't want to
keep your settings and pairing key (`~/.config/keyhop`, or `%APPDATA%\keyhop`).

Settings written to the keyboard (polling rate, keys, lighting) stay in the
keyboard; change them back with `keyhop device` or the vendor's app.
