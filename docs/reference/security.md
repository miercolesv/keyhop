# Security model

## Keyboard/mouse sharing and file transfer

* Every connection uses **Noise XX (X25519, ChaCha20-Poly1305, BLAKE2s)** via
  the `snow` crate, with the prologue `keyhop/1`. Connections are mutually
  authenticated and encrypted; keystrokes, mouse movement and files are never
  sent in plaintext.
* Pairing shows a **6-digit code on both screens**. Confirming that they match
  defeats man-in-the-middle attacks on the local network. After pairing, only
  that computer's public key is accepted; any other connection is dropped right
  after the handshake.
* This computer's private key is stored in `identity.key` in the config folder
  with mode 0600. Paired keys are listed in `trusted` next to it.
* Received file names are sanitised, so files can't be written outside
  `receive_dir`. Partial files are deleted when a transfer fails.
* To restrict which network keyhop listens on, set `listen` to that
  interface's IP address.

## Local interfaces

* Terminal hints use a Unix socket, `hints.sock`, in the config folder, created
  with mode 0600 so only your user can write to it. A hint line can only light
  keyboard keys.
* keyhop makes no outgoing connections except to the paired computer, sends no
  telemetry and has no updater.

## Keyboard writes

* `keyhop device` writes only settings the manufacturer's app can also change.
  It never updates firmware and refuses to write to keyboards whose firmware
  hasn't been verified.
* Hint lighting (accent menu, terminal) changes the keyboard's memory only and
  never saves it, so unplugging the keyboard always restores the saved lighting.
* Factory/test commands found in the firmware are never sent.
