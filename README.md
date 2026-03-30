# SighFar

SighFar is an offline cipher workbench prototype for layered message obfuscation, paired-key recovery, and encrypted local history.

This first pass focuses on a terminal interface because the workspace is empty and the local machine has Swift available today. The product direction still supports a future SmileOS-inspired GUI.

## What this prototype does

- Encodes and decodes messages through a user-chosen chain of techniques
- Supports Morse, Caesar, Vigenere, Rail Fence, and Reverse transforms
- Optionally wraps the transformed output in a paired-key secure envelope
- Stores an encrypted history log locally so it is only readable through the app workflow
- Provides a stylized retro shell that can evolve into a full GUI later

## Important security note

Classical ciphers like Morse, Caesar, Rail Fence, and Vigenere are obfuscation tools, not strong modern cryptography on their own.

In this prototype, "hard to decode" comes from two layers:

1. User-chosen transform chains that make casual inspection harder
2. An AES-GCM secure envelope unlocked by a passphrase plus a separate companion code

For a production-grade cross-platform release, the next step should be replacing any platform-specific crypto assumptions with a portable audited crypto library and moving secrets into OS-backed secure storage.

## Run locally

```bash
swift run
```

## Verification

- `swift build` succeeds in this environment
- the built binary launches and renders the interactive shell
- automated tests are not wired yet because the current local Swift toolchain lacks a usable test framework in this workspace

## Current architecture

- `Sources/SighFar/App.swift`
  The interactive application loop and user flows.
- `Sources/SighFar/CipherPipeline.swift`
  The transformation stack and individual cipher implementations.
- `Sources/SighFar/SecureEnvelope.swift`
  The paired-key authenticated encryption layer.
- `Sources/SighFar/HistoryStore.swift`
  The encrypted on-disk history store.
- `Sources/SighFar/TerminalUI.swift`
  The retro terminal presentation shell.

## Product direction from your idea

### 1. Core workflow

- Enter plaintext
- Choose one or more transform layers
- Optionally apply the secure paired-key envelope
- Share the payload and key parts together or separately
- Decode only when the correct combination is presented

### 2. Encrypted in-app history

The prototype already encrypts history at rest inside `~/.sighfar`.

Production upgrade path:

- macOS: Keychain
- Windows: Credential Locker / DPAPI
- Linux / FreeBSD: Secret Service or encrypted key file fallback
- Android: Keystore

### 3. GUI direction

The desired aesthetic is a retro industrial console similar to SmileOS from Ultrakill:

- heavy red title bars
- dark panel bodies
- chunky framed widgets
- noisy low-fi presentation instead of minimal flat UI

Recommended GUI branches after the workflow is stable:

- Rust + egui for the strongest path toward desktop and Android parity
- Flutter for the fastest polished GUI, though FreeBSD support becomes weaker
- Swift front-end only if the project narrows primarily to Apple platforms

### 4. File hiding mode

This is not implemented yet, but there are two viable paths:

- Carrier container mode: append encrypted payload bytes plus metadata marker to another file
- Real steganography mode: hide bits inside image/audio structures

The first is much easier to ship reliably. The second is more covert but much more format-specific.

### 5. GitHub OAuth

Not implemented in this prototype. The settings module includes a placeholder because this should remain optional and not compromise offline-first usage.

Use OAuth only for:

- syncing profiles or presets
- release channel access
- backup/export workflows

Keep all encode/decode features fully offline.

### 6. Updating instead of duplicate installs

On macOS, replacing an `.app` in `/Applications` is usually handled by app bundle identity:

- keep the same bundle identifier
- keep the same app name
- increment versions properly
- sign and notarize releases consistently

If those stay stable, dragging a newer DMG build into `/Applications` replaces the old app instead of creating a second app.

That part belongs to packaging and release engineering rather than the cipher engine itself.
