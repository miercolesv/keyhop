# Smooth scrolling

Mice with a notched wheel scroll in visible jumps. With smooth scrolling on,
keyhop turns each notch into a short glide that eases out. Trackpads and other
already-smooth sources are left alone.

## Enable

```ini
smooth_scroll = on
scroll_ms = 160        # glide length in milliseconds (16–2000)
scroll_pixels = 48     # macOS only: distance per notch in pixels (1–1000)
```

Restart keyhop after changing these.

## Tuning

- **Too sluggish?** Lower `scroll_ms` (for example `100`).
- **Too far per notch (macOS)?** Lower `scroll_pixels`.
- Turning the wheel the other way cancels the remaining glide immediately.

## How it works

| OS | Method |
|---|---|
| macOS | Wheel events are replaced with pixel-precise scroll events. |
| Linux | The mouse is read with evdev and replayed through a virtual device with high-resolution wheel events, so it works on X11 and Wayland. |
| Windows | Wheel deltas are split into sub-notch steps; precision touchpads are left alone. |

::: warning Mouse software
If your mouse vendor's software also smooths scrolling, turn one of them off,
or scrolling will feel doubled.
:::
