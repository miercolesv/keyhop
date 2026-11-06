# Packaging files

| File | Used for |
|---|---|
| `99-keyhop-uinput.rules` | Linux udev rule: lets the `input` group create virtual input devices and use the raw HID interface of supported keyboards. See `docs/guide/installation.md`. |
| `keyhop-hints.zsh` | zsh hook for terminal hints. `keyhop setup` installs it (it is embedded in the binary); you can also `source` it from `~/.zshrc` after `compinit`. |
