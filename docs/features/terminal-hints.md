# Terminal hints <Badge type="info" text="zsh" />

Like the on-screen keyboard of a car navigation system: while you type in the
terminal, the keys that can come next in the current word light up. The
candidates come from **zsh's own completion system**, so they match what Tab
would offer.

| You type | Keys that light |
|---|---|
| `cd ` | first letters of the folders here |
| `cd Dow` | `n` (Downloads) |
| `git ` | first letters of git's subcommands |
| `git c` | `h`, `l`, `o` (checkout, clone, commit…) |
| `ls -` | the option letters |

The keyboard dims, the possible keys turn **blue** and the others **white**.
Pressing Enter (or `Ctrl+C`) restores your lighting. Keys are only lit, never
blocked.

## Enable

`keyhop setup` sets this up when you answer yes. By hand:

1. In the config file:
   ```ini
   terminal_hints = on
   ```
2. In `~/.zshrc`, **after** `compinit` (oh-my-zsh and similar frameworks run it):
   ```sh
   source /path/to/keyhop-hints.zsh
   ```
   `keyhop setup` installs this file into the config folder. It's also in the
   repository under `packaging/`.
3. Restart keyhop and open a new terminal window.

Requires a [supported keyboard](/hardware/keychron-v1-ultra-8k).

## Details

- Completion runs once when a word starts (and again on its first character,
  because options only appear after `-`). After that the list is just narrowed,
  so typing stays fast even with slow completions like git's.
- Nothing lights at an empty prompt, where every letter would be possible.
- Without `compinit`, hints fall back to command names for the first word and
  files and folders after that.
- When you switch to another app, keyhop notices that keys arrive without a hint
  update and switches the hint off after a few presses.
- The hook talks to keyhop through a socket in the config folder that only
  your user can use. If keyhop isn't running, the hook does nothing.
- Your normal Tab completion is unaffected.
