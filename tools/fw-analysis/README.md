# Firmware analysis scripts

Development-only helpers used to read facts out of a keyboard firmware image
(USB descriptors, HID report layouts, default key bindings). They are not part
of keyhop, and no firmware is included in this repository: pass the path to an
image you have downloaded yourself.

| Script | What it does |
|---|---|
| `scan.py <image>` | finds USB/HID descriptors in the image |
| `hiddesc.py <image> <offset>` | decodes a HID report descriptor at an offset |
| `keymap.py <image>` | infers which HID/consumer usages the default keymap sends |
