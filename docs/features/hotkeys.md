# App-launch hotkeys

Bind a key or key combination to a command. The bound key is used only by
keyhop: on macOS and Windows it doesn't reach the focused app.

## Set up

1. In your keyboard's configuration app (or VIA), map the keys you want to use
   to **F13–F20**. macOS has no F21–F24 keycodes; Linux and Windows support
   F13–F24. Any other key or combination works too.
2. Find the key's name: run `keyhop keys` and press it.
3. Add one `bind` line per hotkey to the [config file](/reference/configuration):

```ini
bind f13 = open -a Safari                     # macOS
bind f14 = firefox                            # Linux
bind f15 = cmd /c start "" "C:\Program Files\App\app.exe"   # Windows
bind ctrl+shift+f16 = code
bind playpause = open -a Music
```

4. Restart keyhop.

## Syntax

```
bind <hotkey> = <command>
```

**Hotkey:** modifiers and one key joined with `+`, in any order, case-insensitive.

| Modifier | Also written as |
|---|---|
| `ctrl` | `control` |
| `shift` | |
| `alt` | `option`, `opt` |
| `meta` | `cmd`, `command`, `super`, `win` |

The key is any name from [Key names](/reference/key-names), including media keys
such as `playpause` or `volumeup`. A hotkey matches only with exactly those
modifiers held: `bind f13` doesn't fire for `ctrl+f13`.

**Command:** a program and its arguments. Quote arguments that contain spaces
(`"double"` or `'single'`). The command runs directly, not through a shell, so
pipes, `&&` and variables need an explicit shell:

```ini
bind f17 = sh -c 'date >> ~/log.txt'
```

## Platform notes

- **macOS, Windows:** the bound key is swallowed.
- **Linux:** the key also reaches the focused app, but F13 and above normally do
  nothing there.
