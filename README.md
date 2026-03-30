# SighFar

SighFar is an offline cipher workbench prototype for layered message obfuscation, paired-key recovery, and encrypted local history.

This branch, `codex/rust-pivot`, moves the project toward a Rust-first implementation because that is the stronger path to a real cross-platform release across macOS, Linux, Windows, Android, and potentially FreeBSD. The interface is still a retro terminal shell for now, with the SmileOS-inspired GUI remaining the next visual milestone.

## What this branch does

- Encodes and decodes messages through a user-chosen chain of techniques
- Supports Morse, Caesar, Vigenere, Rail Fence, and Reverse transforms
- Wraps transformed output in an optional paired-key secure envelope
- Stores an encrypted history log locally so it is only readable through the app workflow
- Establishes a Rust crate layout that can later grow into an egui-based GUI

## Important security note

Classical ciphers like Morse, Caesar, Rail Fence, and Vigenere are obfuscation tools, not strong modern cryptography on their own.

In this Rust pivot, "hard to decode" comes from two layers:

1. User-chosen transform chains that make casual inspection harder
2. An AES-GCM secure envelope unlocked by a passphrase plus a separate companion code, with Argon2-based key derivation

For a production-grade release, the next step is moving secrets into OS-backed secure storage and tightening the file-hiding and GUI layers around the same core workflow.

## Run locally

```bash
cargo run
```

## Verification

- this branch is not compiled in the current workspace because `cargo` and `rustc` are not installed here
- the source tree is structured as a real Rust crate and is ready for build verification on a machine with Rust installed
- `main` still contains the previously verified Swift prototype if you need a runnable local baseline today

## Current Rust architecture

- `src/app.rs`
  Interactive flows and menu handling.
- `src/cipher.rs`
  Cipher chaining and encode/decode implementations.
- `src/secure.rs`
  AES-GCM secure envelope with Argon2-based key derivation from split key material.
- `src/history.rs`
  Encrypted history persistence in `~/.sighfar`.
- `src/ui.rs`
  Retro ANSI terminal shell.

## Product direction from your idea

### 1. Core workflow

- Enter plaintext
- Choose one or more transform layers
- Optionally apply the secure paired-key envelope
- Share the payload and key parts together or separately
- Decode only when the correct combination is presented

### 2. Encrypted in-app history

The Rust pivot keeps the encrypted local history design inside `~/.sighfar`.

Production upgrade path:

- macOS: Keychain
- Windows: Credential Locker or DPAPI
- Linux / FreeBSD: Secret Service or encrypted key file fallback
- Android: Keystore

### 3. GUI direction

The desired aesthetic is a retro industrial console similar to SmileOS from Ultrakill:

- heavy red title bars
- dark panel bodies
- chunky framed widgets
- noisy low-fi presentation instead of minimal flat UI

Recommended GUI branch after the workflow is stable:

- Rust + egui for the strongest path toward desktop and Android parity
- optional Tauri shell if web-style UI composition becomes useful later

### 4. File hiding mode

This is not implemented yet, but there are two viable paths:

- Carrier container mode: append encrypted payload bytes plus metadata marker to another file
- Real steganography mode: hide bits inside image or audio structures

The first is much easier to ship reliably. The second is more covert but much more format-specific.

### 5. GitHub OAuth

Not implemented in this branch. The settings module still includes a placeholder because this should remain optional and not compromise offline-first usage.

Use OAuth only for:

- syncing profiles or presets
- release channel access
- backup or export workflows

Keep all encode/decode features fully offline.

### 6. Updating instead of duplicate installs

On macOS, replacing an `.app` in `/Applications` is usually handled by app bundle identity:

- keep the same bundle identifier
- keep the same app name
- increment versions properly
- sign and notarize releases consistently

If those stay stable, dragging a newer DMG build into `/Applications` replaces the old app instead of creating a second app.

That part belongs to packaging and release engineering rather than the cipher engine itself.
