# Changelog

All notable changes to this project are listed here. The format follows
"Keep a Changelog", and versions follow semantic versioning.

## [0.1.0] - Unreleased

First release.

### Added

- App-launch hotkeys on macOS, Linux (X11 and Wayland) and Windows.
- Smooth scrolling for notched mouse wheels.
- Keyboard/mouse sharing between two computers over Noise XX, with 6-digit
  pairing codes, edge switching (macOS, Windows) and a switch hotkey.
- Encrypted multi-file transfer (`keyhop send`).
- `keyhop device` for the Keychron V1 Ultra 8K: polling rate, debounce, sleep
  timers, snap-click pairs, single keys (including tap-hold) and lighting
  effect/speed.
- Accent-menu lights (macOS): holding a letter lights the number keys that pick
  an accent.
- Terminal hints (zsh): lights the keys that can come next, using zsh's own
  completion.
- `keyhop setup`: guided first-time setup, login service, and macOS permission
  prompts.
- Documentation site (VitePress) and pre-built binaries for macOS (universal),
  Linux (x86_64, aarch64; static) and Windows (x86_64).
