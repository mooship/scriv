# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About

Scriv is a fast local CLI note manager written in Rust (edition 2024) with optional password encryption.

## Commands

```sh
cargo build         # build binary
cargo install --path .  # build and install to Cargo bin dir
cargo test          # run all tests
cargo test add_note_assigns_id_1_when_empty  # run a single test
cargo clippy -- -D warnings   # lint
cargo fmt           # format code (default rustfmt settings)
```

## Safety

- **Never publish (`cargo publish`) or run destructive storage operations without explicit permission from the user.** Always ask first and wait for confirmation.

## Code style

- **No inline comments** — never use `//` comments on the same line as code. Use `///` documentation comments where genuinely useful.
- Use `cargo fmt` (default rustfmt settings) before committing.
- `cargo clippy -- -D warnings` must pass.

## Testing

Tests are unit tests within `src/` modules. Run all with `cargo test`. Tests use `set_notes_path_override(...)` (from `src/storage.rs`) to redirect storage to a temp file, isolating tests from real user data.

## Architecture

`scriv` is a Rust CLI crate with binary + library split:

- **`src/main.rs`** - CLI entry point, command parsing, terminal I/O, and command dispatch
- **`src/lib.rs`** - crate API and re-exports
- **`src/model.rs`** - `Note` and `ListOptions`
- **`src/storage.rs`** - notes path resolution, persistence, active password state
- **`src/crypto.rs`** - encryption/decryption helpers
- **`src/notes.rs`** - core note operations (`add_note`, `remove_notes`, `search_notes`, `clear_notes`, etc.)
- **`src/format.rs`** - display/search helpers (`note_age`, `highlight_match`, `read_stdin_text`)

`src/main.rs` calls into `src/lib.rs`. Keep terminal concerns in the binary and core logic in the library.

### Storage

Notes persist as NDJSON at a platform-specific path resolved by `notes_path()` in `src/storage.rs` (re-exported by `src/lib.rs`).

### ID assignment

IDs are not sequential integers from a counter - new notes get `max(existing IDs) + 1`. IDs are stable after deletion (gaps are preserved).

### Backwards compatibility

This crate is published on crates.io and consumed as a library. All public API in `src/lib.rs` must follow semver:

- **Patch releases** (1.1.x): bug fixes and internal changes only. No changes to public function signatures, return types, or observable behavior (e.g., a function that previously returned `Ok` must not start returning `Err` for the same inputs).
- **Minor releases** (1.x.0): new public functions or fields are OK. Existing signatures and behavior must not break.
- **Major releases** (x.0.0): required for any breaking change to public API (changed return types, removed functions, changed error conditions).

`notes.json` is user data that persists across app versions. Never rename or remove existing JSON keys on `Note` (`id`, `text`, `created_at`, `updated_at`, `tags`). New optional fields must use serde defaults/skip-serialization behavior to preserve compatibility.
