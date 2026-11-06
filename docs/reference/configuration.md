# Configuration

keyhop reads one text file:

| OS | Path |
|---|---|
| macOS, Linux | `~/.config/keyhop/keyhop.conf` (or `$XDG_CONFIG_HOME/keyhop/keyhop.conf`) |
| Windows | `%APPDATA%\keyhop\keyhop.conf` |

`keyhop setup` or `keyhop init` creates it with comments. Restart keyhop after
editing it.

## Format

- One setting per line: `key = value`.
- `#` starts a comment, except inside quotes.
- On/off settings accept `on`, `true`, `yes`, `1` and `off`, `false`, `no`, `0`.
- Unknown keys are an error, so typos don't go unnoticed.
- Paths may start with `~/`.

```ini
# keyhop configuration
peer = 192.168.1.20:24801
peer_edge = right
smooth_scroll = on
accent_lights = on
bind f13 = open -a Safari
```

## Sharing

### `listen`

- **Default:** `0.0.0.0:24801`
- Address and port keyhop listens on for the paired computer. Use this
  computer's LAN address (for example `192.168.1.10:24801`) to accept
  connections on one network only.

### `peer`

- **Default:** empty
- The other computer, as `host:port`. Leave empty to only accept incoming
  connections.

### `peer_edge`

- **Default:** `off`
- **Values:** `left`, `right`, `top`, `bottom`, `off`
- The screen edge that leads to the other computer. Moving the cursor through it
  switches control. macOS and Windows only; on Linux use `switch_hotkey`.

### `switch_hotkey`

- **Default:** `ctrl+alt+f19`
- A [hotkey](/features/hotkeys#syntax) that moves control to the other computer
  and back. `off` disables it.

## File transfer

### `receive_dir`

- **Default:** `~/Downloads/keyhop`
- Where files sent with `keyhop send` arrive.

## Smooth scrolling

### `smooth_scroll`

- **Default:** `off`
- Turns notched wheel clicks into a smooth glide. See
  [Smooth scrolling](/features/smooth-scrolling).

### `scroll_ms`

- **Default:** `160` · **Range:** `16`–`2000`
- Length of the glide in milliseconds.

### `scroll_pixels`

- **Default:** `48` · **Range:** `1`–`1000`
- macOS only: scroll distance per wheel notch, in pixels.

## Key hints

### `accent_lights`

- **Default:** `off`
- macOS: lights the number keys of the accent menu while you hold a letter.
  See [Accent-menu lights](/features/accent-lights).

### `terminal_hints`

- **Default:** `off`
- Lights the keys that can come next while typing in zsh. Needs the zsh hook.
  See [Terminal hints](/features/terminal-hints).

## Hotkeys

### `bind`

- **Syntax:** `bind <hotkey> = <command>`, one line per hotkey.
- Runs a command when the hotkey is pressed. See
  [App-launch hotkeys](/features/hotkeys).

## Files next to the config

| File | Purpose |
|---|---|
| `identity.key` | This computer's private key (mode 0600). |
| `trusted` | Public keys of paired computers. |
| `hints.sock` | Socket for terminal hints, while keyhop runs (mode 0600). |
| `keyhop-hints.zsh` | The zsh hook, installed by `keyhop setup`. |
