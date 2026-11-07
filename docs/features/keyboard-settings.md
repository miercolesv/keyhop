# Keyboard settings

`keyhop device` reads and changes settings stored **in the keyboard**, over the
same configuration interface the manufacturer's app uses. Settings stay in the
keyboard, so they work on every computer, even without keyhop.

::: info Supported keyboards
Writes are allowed only on firmware that has been analysed and verified:
currently the [Keychron V1 Ultra 8K](/hardware/keychron-v1-ultra-8k)
(`3434:0C14`). keyhop never updates firmware and never sends factory commands.
:::

## Show the current settings

```sh
keyhop device
```

```text
keyboard   3434:0C14  firmware v1.0.0-ZK Jul  9 2026 11:22:48
lighting   effect 19, speed 128
rate       8000 Hz wired, 8000 Hz wireless
debounce   press 0 release 50 (type 1)
sleep      backlight off 600 s, sleep 7200 s
snap 1     a + d  (last)
snap 2     off
...
```

Every command below changes one setting and then prints this overview again.

## Polling rate

```sh
keyhop device rate 1000
```

How often the keyboard reports to the computer. Accepted values: `8000`,
`4000`, `2000`, `1000`, `500`, `250`, `125` Hz. Sets the wired rate and keeps
the wireless one. Lower rates use less power and CPU; higher rates lower input
latency.

To set the wireless rate as well, give it as a second value:

```sh
keyhop device rate 8000 1000   # wired 8000 Hz, wireless 1000 Hz
```

The wireless rate accepts `8000`, `4000`, `2000`, `1000` and `500` Hz. The
keyboard applies it in 2.4 GHz mode. A lower wireless rate saves battery.

::: tip Hidden in the vendor app
On the V1 Ultra 8K the manufacturer's app doesn't show the polling rate or the
sleep timers, although the firmware supports both.
:::

## Sleep timers

```sh
keyhop device sleep 600 7200
```

Backlight-off time and sleep time, in seconds. The firmware requires at least
5 s for the backlight and 60 s for sleep.

## Debounce

```sh
keyhop device debounce 0 50
```

Press and release debounce, as the firmware takes them: press `0`–`50`, release
`1`–`50`. The exact unit isn't documented by the firmware; change it in small
steps and test for chatter (double letters).

## Snap-click (SOCD)

Decides what happens when two opposite keys are held at once (for example `a`
and `d` in games). Five pairs are available.

```sh
keyhop device snap 1 last a d     # pair 1: a and d, last pressed wins
keyhop device snap 1 off          # clear pair 1
```

| Type | When both keys are held |
|---|---|
| `last` | the last pressed key wins |
| `first` | the first key of the pair always wins |
| `second` | the second key of the pair always wins |
| `cancel` | neither key is sent |

Keys are [key names](/reference/key-names) of ordinary keys (letters, digits,
arrows…).

## Lighting

```sh
keyhop device effect 19      # choose an effect (1–24), saved
keyhop device speed 128      # effect speed (0–255), saved
```

Known effects on the V1 Ultra 8K:

| Effect | Look |
|---|---|
| `19` | reactive: keys are dark and light up around the key you press |
| `23` | per-key colours |
| `24` | animated rainbow |

Other numbers are other built-in animations; try them and pick one. For the
reactive effects, a higher speed makes the ripple fade sooner, so fewer
neighbouring keys light up.

## Remapping single keys

```sh
keyhop device key 0 3 0              # read: layer 0, row 3, column 0
keyhop device key 0 3 0 0x2129       # write: Caps Lock = tap Esc, hold Ctrl
```

Arguments are layer, row, column and, to write, a keycode. Every write prints
the command that undoes it. The [hardware page](/hardware/keychron-v1-ultra-8k#key-positions)
shows which row and column each key is at.

The keycode is a [key name](/reference/key-names) for ordinary keys, or a QMK
keycode in hex for everything else:

| Keycode | Meaning |
|---|---|
| `0x2129` | tap **Esc**, hold **Ctrl** |
| `0x2829` | tap **Esc**, hold **Cmd/Win** |
| `0x2000 + (mod << 8) + key` | tap `key`, hold modifier `mod` (`MT`) |
| `0x4000 + (layer << 8) + key` | tap `key`, hold for `layer` (`LT`) |
| `0x5220 + layer` | layer while held (`MO`) |
| `0x5260 + layer` | toggle layer (`TG`) |
| `0x5280 + layer` | one-shot layer (`OSL`) |
| `0x52A0 + mod` | one-shot (sticky) modifier (`OSM`) |

`mod` is a QMK modifier mask: `1` Ctrl, `2` Shift, `4` Alt, `8` Cmd/Win, plus
`16` for the right-hand key. Tap-hold keys use only one modifier.

The keyboard has four layers: `0` macOS base, `1` macOS Fn, `2` Windows base,
`3` Windows Fn (chosen by the Mac/Win switch).

::: details Example: keep Caps Lock on Fn + Caps
```sh
keyhop device key 0 3 0 0x2129     # Caps = tap Esc, hold Ctrl
keyhop device key 1 3 0 capslock   # Fn + Caps = Caps Lock
```
:::
