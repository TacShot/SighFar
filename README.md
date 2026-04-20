# SighFar

SighFar is an offline cipher workbench for layered message obfuscation, asymmetric key management, and encrypted local history — built in Rust with a desktop GUI and a terminal fallback.

[![Build](https://github.com/TacShot/SighFar/actions/workflows/build.yml/badge.svg)](https://github.com/TacShot/SighFar/actions/workflows/build.yml)

## Features

- **Cipher chain builder** — stack Morse, Caesar, Vigenère, Rail Fence, Reverse, SHA-256, and SHA-512 transforms in any order
- **AES-256-GCM secure envelope** — wrap output with a passphrase + auto-generated companion code using Argon2 key derivation
- **RSA-2048 encrypt / decrypt** — asymmetric encryption with OAEP/SHA-256 padding
- **Automatic key management** — RSA key pairs are auto-generated and stored in an encrypted local database (`~/.sighfar/keys.enc`); manage them from the History & Keys tab
- **Encrypted history** — every encode/decode operation is logged in an AES-256-GCM encrypted file at `~/.sighfar/history.enc`
- **Carrier file mode** — hide any file inside another by appending an extractable SighFar trailer
- **GitHub device-flow sync** — push/pull the encrypted history blob to a private GitHub repository
- **Desktop GUI** — retro industrial console style built with [egui](https://github.com/emilk/egui) with animated tab transitions
- **Terminal fallback** — full encode/decode/history workflow available via `--tui`

## Security notes

Classical ciphers (Morse, Caesar, Rail Fence, Vigenère) are obfuscation tools, not cryptographic primitives.  "Hard to decode" comes from two layers:

1. User-chosen transform chains that make casual inspection harder
2. An AES-256-GCM secure envelope (or RSA-OAEP envelope) unlocked by key material only held by the intended recipient

SHA-256 and SHA-512 in the cipher chain produce a one-way digest — they cannot be decoded.

## Supported ciphers

| Technique | Chain key | Notes |
|---|---|---|
| Morse | `morse` | encodes/decodes alphanumeric + space |
| Caesar | `caesar:N` | rotate by N (−25 to 25) |
| Vigenère | `vigenere:keyword` | keyword must contain letters |
| Rail Fence | `railfence:N` | N ≥ 2 rails |
| Reverse | `reverse` | reverses the character sequence |
| SHA-256 | `sha256` | one-way hex digest; encode only |
| SHA-512 | `sha512` | one-way hex digest; encode only |

RSA encrypt/decrypt is available as a separate operation in the GUI and through `SighFarCore::rsa_encrypt` / `rsa_decrypt`.

## Run locally

```bash
cargo run
```

Terminal fallback:

```bash
cargo run -- --tui
```

## Tests

```bash
cargo test
```

Seven tests covering the cipher pipeline round-trip, Morse encoding, AES-GCM secure envelope, carrier file embed/extract, RSA key generation, and encrypted history persistence.

## Architecture

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point — GUI or `--tui` |
| `src/core.rs` | Shared workflow logic for encode, decode, RSA, and key management |
| `src/cipher.rs` | Cipher chain: Morse, Caesar, Vigenère, Rail Fence, Reverse, SHA-256, SHA-512 |
| `src/secure.rs` | AES-256-GCM envelope with Argon2 key derivation |
| `src/keys.rs` | Auto RSA-2048 key-pair management with encrypted on-disk storage |
| `src/history.rs` | Encrypted history persistence in `~/.sighfar` |
| `src/gui.rs` | egui desktop shell with animated tabs, RSA panel, and key manager |
| `src/app.rs` | Terminal (TUI) flow |
| `src/ui.rs` | ANSI terminal shell helper |
| `src/carrier.rs` | Carrier container format for file-in-file embedding |
| `src/github_sync.rs` | GitHub device-flow OAuth and encrypted history push/pull |
| `src/config.rs` | Lightweight local config for GitHub sync settings |
| `src/models.rs` | Shared data types |

## Builds

Automated builds run on every push via GitHub Actions for:

- **Linux** — `x86_64-unknown-linux-gnu`
- **macOS** — `x86_64-apple-darwin`
- **Windows** — `x86_64-pc-windows-msvc`

Download the latest build artifacts from the [Actions tab](../../actions).

## Local key storage paths

| Platform | History | Key store |
|---|---|---|
| macOS / Linux | `~/.sighfar/history.enc` | `~/.sighfar/keys.enc` |
| Windows | `%USERPROFILE%\.sighfar\history.enc` | `%USERPROFILE%\.sighfar\keys.enc` |

The encryption keys for both stores are held in adjacent `.key` files.  The GitHub sync feature pushes only the encrypted history blob — never the key files.

