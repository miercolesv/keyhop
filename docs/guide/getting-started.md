# Getting started

keyhop is one small native program, configured with a commented text file. This
page gets you from zero to a running setup in a few minutes.

## 1. Get keyhop

**Download a pre-built binary** for your platform from the
[releases page](https://github.com/miercolesv/keyhop/releases), unpack it, and
continue with step 2 using that file.

**Or build it yourself.** You need a Rust toolchain (install it with rustup); no
other build tools are required.

```sh
git clone https://github.com/miercolesv/keyhop.git
cd keyhop
cargo build --release
```

The binary is `target/release/keyhop` (`keyhop.exe` on Windows).

::: tip macOS: first launch of a downloaded binary
Downloaded binaries aren't notarised, so macOS may refuse to open them. Remove
the download flag once: `xattr -d com.apple.quarantine ./keyhop`
:::

## 2. Run the guided setup

```sh
./keyhop setup          # or ./target/release/keyhop setup if you built it
```

Setup asks a few yes/no questions and then:

- copies the binary to `~/.local/bin/keyhop` (`%LOCALAPPDATA%\keyhop` on Windows),
- writes your settings to the [config file](/reference/configuration),
- offers the [keyboard features](/features/keyboard-settings) if a supported
  keyboard is connected,
- installs the [terminal hints](/features/terminal-hints) hook if you use zsh,
- starts keyhop at login.

::: tip Run it again any time
`keyhop setup` is safe to repeat. It updates your existing config instead of
replacing it, and pressing Enter keeps your current answer.
:::

## 3. Allow keyhop to see the keyboard

::: code-group

```text [macOS]
macOS shows two prompts for the keyhop binary.
Switch it on under System Settings → Privacy & Security →
Accessibility and Input Monitoring. keyhop starts working
within about 10 seconds.
```

```sh [Linux]
# once, then log out and back in
sudo usermod -aG input "$USER"
sudo cp packaging/99-keyhop-uinput.rules /etc/udev/rules.d/
sudo modprobe uinput && echo uinput | sudo tee /etc/modules-load.d/uinput.conf
sudo udevadm control --reload && sudo udevadm trigger
```

```text [Windows]
Nothing to do.
```

:::

## 4. Check that it works

```sh
keyhop keys
```

Press a few keys: each one prints its name. These are the names the config uses.
Press `Ctrl+C` to stop.

If you have a supported keyboard:

```sh
keyhop device
```

shows its firmware, lighting, polling rate and other settings.

## Next steps

- Bind keys to apps: [App-launch hotkeys](/features/hotkeys)
- Control a second computer: [Keyboard and mouse sharing](/features/sharing)
- Every setting explained: [Configuration reference](/reference/configuration)
- Something not working? [Troubleshooting](/guide/troubleshooting)
