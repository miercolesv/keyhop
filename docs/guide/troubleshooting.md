# Troubleshooting

## Where are the logs?

| OS | Running as a service | Running in a terminal |
|---|---|---|
| macOS | `~/Library/Logs/keyhop.log` | printed to the terminal |
| Linux | `journalctl --user -u keyhop -f` | printed to the terminal |
| Windows | — | printed to the console (`keyhop run`) |

Restart the service after changing the config:

::: code-group

```sh [macOS]
launchctl kickstart -k gui/$(id -u)/dev.keyhop
```

```sh [Linux]
systemctl --user restart keyhop
```

:::

## keyhop doesn't react to any key

**macOS: "cannot watch the keyboard yet".** keyhop lacks Accessibility or Input
Monitoring. Open System Settings → Privacy & Security and switch keyhop on in
both lists. macOS ties the permission to the exact binary, so after installing
a new build you may have to switch it off and on again.

**macOS: nothing happens in password fields.** That's Secure Input: macOS hides
keystrokes from every app like keyhop while a password field has focus.

**Linux: "permission denied" on `/dev/input` or `/dev/uinput`.** Follow the
Linux steps in [Installation](/guide/installation#permissions), then log out and
back in so the new group applies.

## "no keyboard found with vendor 0x3434 raw HID interface"

- Check that the keyboard is connected. The keyboard features use its USB
  configuration interface and have only been tested over a USB cable.
- On Linux, the udev rule must be installed (it grants access to `hidraw`).
- `keyhop device` only knows [supported keyboards](/hardware/keychron-v1-ultra-8k).

## "refusing to write: firmware … has not been verified"

keyhop only changes settings on firmware whose commands have been checked. You
can still read settings with `keyhop device`. `CONTRIBUTING.md` in the
repository explains how a keyboard gets verified.

## The keyboard stays in a strange lighting pattern

Hint lighting is never saved to the keyboard. Unplug and reconnect it to get
your saved lighting back, or start keyhop again: the next hint restores it.

## Accent-menu lights don't appear

- `accent_lights = on` must be set, and keyhop restarted.
- The accent menu itself must be enabled. Check with
  `defaults read -g ApplePressAndHoldEnabled`: `0` means it's off. Turn it on with
  `defaults write -g ApplePressAndHoldEnabled -bool true` and log out and back in.
- Some apps never show the menu (Terminal, some games and Electron apps).
- The letter must be held for about half a second with no Cmd, Ctrl or Alt.

## Terminal hints don't appear

1. `terminal_hints = on` in the config, and keyhop restarted.
2. Open a **new** terminal window (or run `exec zsh`) so the hook is loaded.
3. The socket must exist: `ls ~/.config/keyhop/hints.sock`.
4. The hook must be sourced **after** `compinit` (oh-my-zsh runs it). Without
   it, hints fall back to command names and files.
5. Nothing lights at an empty prompt: every letter would be possible there.

## Pairing fails

- Stop the service on both computers first; pairing uses the same port.
- Allow TCP port 24801 (or your `listen` port) through the firewall.
- Both screens must show the same 6-digit code. If they don't, someone else is
  in between: answer `n`.

## Smooth scrolling feels doubled

The mouse vendor's software may also smooth scrolling. Turn one of them off.
