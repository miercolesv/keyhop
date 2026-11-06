# Accent-menu lights <Badge type="info" text="macOS" />

macOS types accented letters when you hold a key: a small menu appears and you
pick a variant with a number key. With accent lights on, the keyboard lights
**only the number keys that are valid choices** for the letter you're holding.

| You hold | Keys that light |
|---|---|
| `a`, `o` | 1–8 |
| `e` | 1–7 |
| `i` | 1–6 |
| `u` | 1–5 |
| `n` | 1–2 |
| `c`, `j` | 1 |

The valid numbers light white and every other key turns blue. Your next key
press (the number, or anything else) restores your lighting.

## Enable

```ini
accent_lights = on
```

Requires a [supported keyboard](/hardware/keychron-v1-ultra-8k) and macOS's
accent menu (on by default). Restart keyhop after changing it.

## Details

- The letter must be held for about half a second, with no Cmd, Ctrl or Alt.
  Shift is fine (for capital letters).
- Releasing the letter keeps the lights on, like the menu stays open.
- If no key follows within 10 seconds, the lighting is restored anyway.
- The counts above are those of macOS's menu for English input; other input
  languages may offer different numbers of variants.
- Colours change in the keyboard's memory only and are never saved to it.

::: info Why blue instead of dark?
The keyboard's per-key mode uses one brightness for every key, so single keys
can't be switched off. Contrast comes from colour instead.
:::
