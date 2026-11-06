# Keyboard and mouse sharing

Use one keyboard and mouse for two computers. Move the cursor across a screen
edge, or press a hotkey, and your input goes to the other computer. Everything
travels over an authenticated, encrypted channel (see
[Security model](/reference/security)).

Works between any mix of macOS, Linux and Windows.

## Pair the computers (once)

Pairing exchanges keys so each computer only accepts the other one. Stop the
keyhop service on both computers first, then:

```sh
keyhop pair --listen              # on computer B
keyhop pair 192.168.1.20:24801    # on computer A, using B's address
```

Both screens show a **6-digit code**. If they match, answer `y` on both. If
they don't match, answer `n`: something is between the two computers.

## Configure

In the config file on **each** computer:

```ini
peer = 192.168.1.20:24801   # the other computer
peer_edge = right           # the edge that leads to it
```

For example `right` on the left computer and `left` on the right one. Then start
keyhop (or restart the service).

## Switching

| How | macOS | Linux | Windows |
|---|---|---|---|
| Move the cursor through `peer_edge` | ✅ | — | ✅ |
| Press `switch_hotkey` (default `ctrl+alt+f19`) | ✅ | ✅ | ✅ |

On macOS and Windows the cursor comes back when it crosses back over the edge.
On Linux, press the hotkey again: Wayland doesn't expose the global cursor
position, so edge switching isn't possible there.

## What gets forwarded

- Keys travel as USB HID codes, so any OS combination works. Modifiers keep
  their physical position: Cmd on a Mac acts as Super/Win on the other computer.
- Mouse movement, buttons and the wheel.
- Media keys. Brightness can't be forwarded *to* Windows (it has no brightness
  key), and Mission Control / Launchpad only map between macOS and Linux.
- When control comes back, keys and buttons still held are released on the
  other computer, so nothing stays stuck.

## File transfer

```sh
keyhop send report.pdf photo.jpg
```

The files arrive in `receive_dir` on the other computer (default
`~/Downloads/keyhop`). Names are sanitised, an existing file is never
overwritten (the new one gets a different name), and partial files are deleted
if a transfer fails.

## Managing paired computers

```sh
keyhop trusted          # list paired computers
keyhop untrust <name>   # remove one (by name or fingerprint prefix)
keyhop id               # this computer's key fingerprint
```

## Limits

- One paired computer at a time.
- No clipboard sync, no screen mirroring.
