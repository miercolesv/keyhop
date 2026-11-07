# Commands

Run `keyhop` without arguments to print this list.

## Setup and service

| Command | What it does |
|---|---|
| `keyhop setup` | Guided first-time setup: installs the binary, writes the config, installs the terminal hook and the login service. Safe to repeat. |
| `keyhop init` | Creates the config file (if missing) and this computer's identity key. |
| `keyhop run` | Runs keyhop in the foreground; logs go to the terminal. |
| `keyhop run --background` | Windows: detaches from the console window. Same as `run` elsewhere. |
| `keyhop install-service` | Starts keyhop at login: launchd agent (macOS), systemd user unit (Linux), `HKCU\…\Run` (Windows). |
| `keyhop keys` | Prints the name of every key and button you press (`Ctrl+C` to stop). |

## Sharing and files

| Command | What it does |
|---|---|
| `keyhop pair --listen` | Waits for the other computer to pair (stop the service first). |
| `keyhop pair <host:port>` | Pairs with a computer running `pair --listen`. |
| `keyhop trusted` | Lists paired computers. |
| `keyhop untrust <name>` | Removes a paired computer, by name or fingerprint prefix. |
| `keyhop send <file>...` | Sends files to the configured peer. |
| `keyhop id` | Shows this computer's key fingerprint. |

## Keyboard

See [Keyboard settings](/features/keyboard-settings) for details.

| Command | What it does |
|---|---|
| `keyhop device` | Shows firmware, lighting, polling rate, debounce, sleep and snap-click. |
| `keyhop device rate <hz>` | Wired polling rate: `8000` `4000` `2000` `1000` `500` `250` `125`. |
| `keyhop device rate <hz> <hz>` | Wired and wireless polling rates. Wireless: `8000` `4000` `2000` `1000` `500`. |
| `keyhop device sleep <backlight> <sleep>` | Timeouts in seconds (at least 5 and 60). |
| `keyhop device debounce <press> <release>` | Press `0`–`50`, release `1`–`50`. |
| `keyhop device snap <1-5> <type> <key> <key>` | Snap-click pair; type `last`, `first`, `second` or `cancel`. |
| `keyhop device snap <1-5> off` | Clears a snap-click pair. |
| `keyhop device effect <1-24>` | Lighting effect, saved. |
| `keyhop device speed <0-255>` | Effect speed, saved. |
| `keyhop device key <layer> <row> <col>` | Shows one key's keycode. |
| `keyhop device key <layer> <row> <col> <code>` | Sets one key (name or QMK hex) and prints the undo command. |
