# Keychron V1 Ultra 8K

Facts below were read from the official firmware image
`v1.0.0_2026-07-17_V1_Ultra_8K_ansi_V2_v1.0.0_2607091122_ota.bin` (reported by the
keyboard as "v1.0.0-ZK Jul 9 2026 11:22:48", device version V2). The firmware
itself is not included in or used by keyhop. The facts come only from its USB/HID
descriptors and default key bindings.

## Identity

* USB VID:PID `3434:0C14` (the earlier ANSI revision is `0C10`; ISO is `0C11`/`0C15`
  and JIS is `0C12`).
* Firmware: ZMK on the Zephyr RTOS. Fn layers, knob modes and Bluetooth/2.4 GHz
  profile switching are all handled inside the keyboard, so the computer only
  ever sees the reports below.

## HID reports (USB)

| Report ID | Page | What it carries | keyhop handling |
|---|---|---|---|
| 1 | Keyboard | Boot keyboard, 6-key rollover | keyboard table (`keys.rs`) |
| 4 | Keyboard | NKRO bitmap, usages 0x00–0x97 (includes F13–F24) | same |
| 2 | Consumer (0x0C) | 2 × 16-bit usages, 0x000–0xFFF | `CONSUMER_TABLE` (`keys.rs`) |
| 5 | System Control (0x01/0x80) | power/sleep/wake | not bound by default; ignored |
| 3 | Mouse | 5 buttons, X/Y, wheel, AC Pan | treated like any mouse (knob in mouse mode) |
| — | Vendor 0xFF60 | 32-byte raw HID, used by Keychron Launcher for configuration | not used by keyhop (see below) |
| — | 0x8C usage 1 | 33-byte reports, separate interface; purpose unconfirmed (possibly update/vendor channel) | never touched |

## Raw HID command set (0xFF60), observed on 2026-11-06

The command map comes from the firmware's dispatcher; the values were read from
the live keyboard with read-only queries. Every reply echoes the command bytes.
Unknown commands are answered with `0xFF` in byte 0. No authentication or unlock
step exists in front of the dispatcher.

VIA protocol 12 ("V3"): `01`–`09` and `0C`–`15` are implemented. `0A` (EEPROM
reset) and `0B` (bootloader jump) are not.
Live: 4 layers.

| Cmd | Meaning | Read-only? | Live value |
|---|---|---|---|
| `A0` | Launcher protocol version | yes | `01 00 01` ("KCV1") |
| `A1` | firmware version string | yes | `v1.0.0-ZK Jul  9 2026 11:22:48` |
| `A2` | feature bitmap: **hard-coded constant** in firmware | yes | `E1` |
| `A3` | default layer | yes | 0 |
| `A7 01` | misc protocol version | yes | `00 03` |
| `A7 05` / `06` | debounce get / set (set: two bytes, ≤50 and 1–50, units unconfirmed) | get only | `00 32` |
| `A7 07` | snap-click slot count | yes | 5 |
| `A7 08 start n` | snap-click pairs, 3 bytes each | yes | all empty |
| `A7 09` / `0A` | snap-click set / save to flash | no | — |
| `A7 0B` / `0C` | low-power timers get / set (two u16 LE) | get only | 600, 7200 (seconds, assumed) |
| `A7 0D` / `0E` | report-rate info / set (set: three indexes ≤6, applied immediately) | get only | current index 0, masks `7F`/`1F` |
| `A7 02`–`04` | not implemented in this firmware (no-op) | — | — |
| `A8` | RGB group (not analysed) | — | — |
| `AB` | checksummed factory channel (16-bit sum of bytes 1–28 in bytes 30–31); one sub-command writes a magic to retained RAM and resets the chip | **do not send** | — |

Formats confirmed against Keychron Launcher's own code (2026-11-06):

* Polling rate = 8000 Hz >> index (0 = 8000 … 6 = 125). Set: `A7 0E wired wireless x`.
  The firmware stores `wired` and applies it in wired mode, and `wireless` in mode 2.
  `x` is range-checked (≤ 6) but not used.
* Sleep: little-endian u16 backlight-off and sleep times; the firmware rejects
  backlight < 5 or sleep < 60. Launcher shows these units as seconds.
* Debounce reply: `[3]` type (1 = press/release pair), `[4]` press, `[5]` release.
* Snap-click entry: `[type, keycode A, keycode B]`, type 2 = last key wins,
  3 = first key wins, 4 = second key wins, 5 = both cancelled. Set
  `A7 09 start count entries…`, then `A7 0A` to save.

**Launcher hides working features.** With Launcher protocol ≤ 3 (this keyboard
reports 1), Launcher uses its old feature parser, which hard-codes
`Support_Sleep`, `Support_PollRate`, `Support_Latch` and `Support_Macro_Repeat`
to false. The firmware implements sleep (`A7 0B/0C`) and polling rate
(`A7 0D/0E`), so those two work even though Launcher never offers them. Latch
and macro repeat are not in this firmware's `A7` table.

`A2` = `E1` decodes with Launcher's parser as: default layer, debounce, snap-click
and Keychron RGB supported.

## What the default keymap sends besides normal keys

Consumer usages: brightness up/down (0x06F/0x070), previous/next track
(0x0B6/0x0B5), play/pause (0x0CD), mute (0x0E2), volume up/down (0x0E9/0x0EA,
also the knob), Mission Control (0x29F) and Launchpad (0x2A0).

Shortcut keys are sent as an ordinary key plus GUI: Win/Cmd+Tab (task view),
Win/Cmd+Space and Win/Cmd+E. No F13–F24 are bound by default. Assign them in
Keychron Launcher to use them with `bind`.

## How each OS reports these keys (as keyhop handles them)

| Key | Linux evdev | macOS | Windows |
|---|---|---|---|
| media/volume/brightness | separate "Consumer Control" input node (`KEY_*`) | `NX_SYSDEFINED` aux-button events | `VK_MEDIA_*` / `VK_VOLUME_*` (no brightness VK) |
| Mission Control | `KEY_SCALE` | keycode 0xA0 (unverified) | none |
| Launchpad | not mapped by the kernel | keycode 0x83 (unverified) | none |

## Keymap over VIA (read 2026-11-06)

Matrix 6 × 16, 4 layers: 0 = macOS base, 1 = macOS Fn, 2 = Windows base,
3 = Windows Fn (selected by the Mac/Win switch). Keycodes use QMK numbering
(big-endian in VIA buffers). The firmware translates them into ZMK behaviours:

| QMK range | ZMK behaviour |
|---|---|
| `0x0004–0x00FF` (and mods+key `0x0100–0x1FFF`) | `key_press` |
| `0x2000–0x3FFF` `MT(mod, key)` | `mod_tap` (first modifier bit only) |
| `0x4000–0x4FFF` `LT(layer, key)` | `layer_tap` |
| `0x5000–0x51FF` `LM(layer, mod)` | `layer_mod` |
| `0x5200` TO, `0x5220` MO, `0x5260` TG | `to_layer`, `momentary_layer`, `toggle_layer` |
| `0x5280` OSL, `0x52A0` OSM, `0x52C0` TT | `sticky_layer`, `sticky_key`, `tapd_layer` |
| `0x5240` DF | not supported (`none`) |

Tap-hold timing (tapping term, hold preference) is fixed in the firmware; no
runtime command to change it was found.

## Lighting (verified live 2026-11-06)

* VIA custom values on channel 3 only (`07`/`08`/`09` = set/get/save): value 1
  brightness, 2 effect (1–24), 3 speed, 4 colour (hue, saturation). Set changes
  RAM; `09` saves. Effect 19 = reactive multiwide, 23 = per-key colours,
  24 = an animated rainbow.
* `A8` (KC_RGB): 1 version, 2 save, 3/4 indicator, 5 LED count (82), 6 LED index
  by row (`A8 06 row FF FF FF`), 7/8 per-key effect type 0–4 (0 = static),
  9/10 get/set colours (`start count` then HSV × count, max 9 per report),
  11 info, 12/13 regions, 14/15 mix. Only 2 writes flash.
* Number row = matrix row 1; keys 1–9 and 0 are LEDs 0x10–0x19.
* Brightness writes are stored as `x * 255 >> 8`; writing back a value just
  read loses one step (keyhop compensates, see `brightness_arg`).
* All five per-key renderers use each key's hue and saturation only; V comes
  from the global brightness. A single key cannot be switched off in per-key mode.

## Key positions

Matrix row and column of each key (ANSI, base layer), for
`keyhop device key <layer> <row> <col>`. Columns without a key are left out.

| Row | Col → key |
|---|---|
| 0 | 0 Esc · 1–12 F1–F12 · 13 Delete · 15 knob press |
| 1 | 0 \` · 1–10 1…0 · 11 - · 12 = · 13 Backspace · 15 Page Up |
| 2 | 0 Tab · 1–10 Q…P · 11 [ · 12 ] · 13 \\ · 15 Page Down |
| 3 | 0 Caps Lock · 1–9 A…L · 10 ; · 11 ' · 13 Enter · 15 Home |
| 4 | 0 Left Shift · 2–8 Z…M · 9 , · 10 . · 11 / · 12 Right Shift · 14 Up |
| 5 | 0 Left Ctrl · 1 Left Option/Win · 2 Left Cmd/Alt · 6 Space · 10 Right Cmd · 11 Fn · 12 Right Ctrl · 13 Left · 14 Down · 15 Right |

Layers: `0` macOS base, `1` macOS Fn, `2` Windows base, `3` Windows Fn.
