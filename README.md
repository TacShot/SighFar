# SighFar

SighFar is an offline cipher workbench prototype for layered message obfuscation, paired-key recovery, and encrypted local history.

This branch, `codex/rust-pivot`, moves the project toward a Rust-first implementation because that is the stronger path to a real cross-platform release across macOS, Linux, Windows, Android, and potentially FreeBSD. It now includes a first desktop GUI shell in `egui`, while still keeping the terminal mode available as a fallback.

## What this branch does

- Encodes and decodes messages through a user-chosen chain of techniques
- Supports Morse, Caesar, Vigenere, Rail Fence, and Reverse transforms
- Wraps transformed output in an optional paired-key secure envelope
- Stores an encrypted history log locally so it is only readable through the app workflow
- Adds a SmileOS-inspired desktop shell for encode, decode, history, settings, and roadmap views
- Includes carrier-file mode for hiding one file inside another using an extractable trailer
- Includes GitHub device-flow sign-in so the app can create a private sync repository and push or pull the encrypted history blob
- Keeps a `--tui` fallback if you want the terminal flow

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

Terminal fallback:

```bash
cargo run -- --tui
```

## Verification

- `cargo build` succeeds
- `cargo test` passes with 5 unit tests
- the desktop GUI launches with `cargo run`
- `main` still contains the earlier Swift prototype if you want to compare directions

## Current Rust architecture

- `src/app.rs`
  Terminal fallback flow.
- `src/core.rs`
  Shared workflow logic used by both the GUI and terminal modes.
- `src/cipher.rs`
  Cipher chaining and encode/decode implementations.
- `src/carrier.rs`
  Carrier container format for embedding and extracting one file inside another.
- `src/config.rs`
  Lightweight local config for saved GitHub sync settings.
- `src/gui.rs`
  `egui` desktop shell with the SmileOS-inspired layout, dropdown cipher chain builder, carrier UI, and GitHub sync settings.
- `src/github_sync.rs`
  GitHub device-flow sign-in, private repo creation, and encrypted history push/pull helpers.
- `src/secure.rs`
  AES-GCM secure envelope with Argon2-based key derivation from split key material.
- `src/history.rs`
  Encrypted history persistence in `~/.sighfar`.
- `src/ui.rs`
  Retro ANSI terminal shell used by `--tui`.

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

The desired aesthetic is a retro industrial console similar to SmileOS:

- heavy red title bars
- dark panel bodies
- chunky framed widgets
- noisy low-fi presentation instead of minimal flat UI

Recommended GUI branch after the workflow is stable:

- Rust + egui for the strongest path toward desktop and Android parity
- optional Tauri shell if web-style UI composition becomes useful later

### 4. File hiding mode

This now has an initial carrier-container implementation:

- Carrier container mode: append payload bytes plus metadata trailer to another file and extract them later
- Real steganography mode: hide bits inside image or audio structures

The current implementation is the first path because it is much easier to ship reliably. Real steganography is still a future enhancement.

### 5. GitHub OAuth

This branch now includes a first pass at GitHub sync through the OAuth device flow. The user provides a GitHub OAuth app client ID in Settings, signs in, and the app can create a private repository automatically if it does not exist yet.

Use OAuth only for:

- syncing profiles or presets
- release channel access
- backup or export workflows

The current sync implementation pushes and pulls only the encrypted history blob, not the local key file. Keep all encode/decode features usable offline even when GitHub sync is not configured.

### 6. Updating instead of duplicate installs

On macOS, replacing an `.app` in `/Applications` is usually handled by app bundle identity:

- keep the same bundle identifier
- keep the same app name
- increment versions properly
- sign and notarize releases consistently

If those stay stable, dragging a newer DMG build into `/Applications` replaces the old app instead of creating a second app.

That part belongs to packaging and release engineering rather than the cipher engine itself.
