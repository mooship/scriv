# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. `AGENTS.md` just points here — edit this file, not that one.

## About

Scriv (`Cargo.toml`) is a local CLI note manager, edition 2024, MSRV 1.88, currently at version 2.1.0, licensed GPL-3.0-only (`LICENSE`). Published on crates.io and docs.rs as a library + binary. No network calls, no daemon, no accounts — everything lives in one local file.

Repo is small and flat: `src/` (7 files), `tests/` (4 integration files + `tests/common/`), `.github/` (CI + dependabot), plus `Cargo.toml`/`Cargo.lock`, `README.md`, `LICENSE`, `.gitattributes`, `.gitignore`. No `docs/`, no `rust-toolchain.toml`, no `rustfmt.toml`/`clippy.toml` (both tools run on their defaults), no dev-dependencies section — `tempfile` and `once_cell` are regular dependencies (used by `src/storage.rs`) that the test helpers in `tests/common/mod.rs` reuse.

## Commands

```sh
cargo build              # build binary
cargo install --path .   # build and install to Cargo bin dir
cargo test               # run all tests (integration only — no #[cfg(test)] units in src/)
cargo test add_note_assigns_id_1_when_empty  # run a single test
cargo clippy --all-targets -- -D warnings    # lint (see clippy caveat below)
cargo fmt                # format code (default rustfmt settings)
```

CI (`.github/workflows/ci.yml`) runs, on push to `main`, on every PR, and weekly (Saturday 07:00 UTC): `cargo fmt -- --check`, `cargo clippy --all-features ... -D warnings` (piped through `clippy-sarif`/`sarif-fmt` for GitHub code scanning), `cargo build --locked`, `cargo test --locked`. There's no `[features]` table in `Cargo.toml`, so `--all-features` is a no-op here. CI only runs on `ubuntu-latest` — the Windows/macOS branches of `notes_path()` (`src/storage.rs`) are never exercised by CI, only by reading the code.

**Clippy caveat, verified against this checkout:** the CI clippy step is `cargo clippy ... -D warnings | clippy-sarif | tee ... | sarif-fmt`. None of the CI steps set `shell: bash`, so GitHub Actions runs them as plain `bash -e {0}` *without* `-o pipefail`. That means the step's exit status is whatever `sarif-fmt` returns, not `cargo clippy` — a `-D warnings` failure in clippy will not fail CI. This is not hypothetical: as of this checkout, `cargo clippy --all-targets -- -D warnings` actually fails, with two pre-existing deprecation errors in `src/crypto.rs:31` and `:62` (`Nonce::from_slice` deprecated by the pinned `hybrid-array 0.4.12`, itself pulled in by `aes-gcm 0.11.0`, per `Cargo.lock`). Confirmed via `git stash` that this predates any doc-only change. Don't assume a green CI run means clippy is clean — run it yourself locally and treat `-D warnings` as the real bar.

## Safety

- **Never publish (`cargo publish`) or run destructive storage operations without explicit permission from the user.** Always ask first and wait for confirmation.

## Code Style

- **No inline comments** — never use `//` comments on the same line as code. Use `///` documentation comments where genuinely useful.
- **British English spelling** in user-facing copy (CLI help text, error messages, `README.md`) — "colour", "organise" — code identifiers stay standard English.
- Run `cargo fmt` before committing; there's no `rustfmt.toml`, so behavior can shift slightly with rustfmt version (CI floats `dtolnay/rust-toolchain@stable`, there's no pinned toolchain file).
- `cargo clippy -- -D warnings` must pass locally — CI will not catch it if it doesn't (see caveat above).

## Engineering Principles

- **TDD.** Write the failing integration test in `tests/` first, then the implementation. Anything new that touches storage or the active password follows the `lock_test()`/`TestEnv::new()` pattern below from the start, not bolted on after.
- **DRY.** Route new user-facing output through `sanitize_display` and thread new password/plaintext values through `Zeroizing` — both exist precisely so new code doesn't reimplement escaping or zeroing.
- **KISS / YAGNI.** This is a small, flat, single-user CLI with no daemon and no network calls — prefer a plain function in the matching `src/*.rs` file over a new abstraction layer, and don't add config surface or feature flags for a use case nobody has asked for.

## Testing

Tests are integration tests under `tests/` (`crypto_tests.rs`, `format_tests.rs`, `lib_tests.rs`, `storage_tests.rs`), sharing helpers from `tests/common/mod.rs`. Run all with `cargo test`.

`src/storage.rs` holds process-global mutable state: `NOTES_PATH_OVERRIDE` and `ACTIVE_PASSWORD` (both `Lazy<Mutex<_>>`). `cargo test` runs tests within one binary in parallel threads by default, and all tests in `lib_tests.rs`/`storage_tests.rs` share that same process-wide state. That's why every such test starts with:

```rust
let _guard = lock_test();   // tests/common/mod.rs — global Mutex, serializes these tests
let _env = TestEnv::new();  // redirects notes_path() to a temp file, clears the active password
```

Any new test that touches storage or the active password **must** take `lock_test()` first, or it will race with other tests in the same binary and get spurious failures or cross-test data leakage. `TestEnv::new()`/`Drop` reset both statics; the `_guard` must outlive `_env` (declare it first, as above) since dropping the lock before storage is reset re-opens the race window for the next test.

## Architecture

`scriv` is a Rust crate with a binary + library split — `[[bin]]`/`[lib]` aren't declared explicitly in `Cargo.toml`; having both `src/main.rs` and `src/lib.rs` is what gives Cargo the two implicit targets, both named after the package.

- **`src/main.rs`** - CLI entry point, arg parsing, terminal I/O, command dispatch (`cmd_*` functions)
- **`src/lib.rs`** - crate API and re-exports; the `crypto` re-exports are `#[doc(hidden)]` (internal use by the binary and tests, not part of the intended public surface)
- **`src/model.rs`** - `Note` and `ListOptions`
- **`src/storage.rs`** - notes path resolution, persistence, active password state
- **`src/crypto.rs`** - AES-256-GCM encryption/decryption
- **`src/notes.rs`** - core note operations (`add_note`, `remove_notes`, `search_notes`, `clear_notes`, etc.)
- **`src/format.rs`** - display/search helpers (`note_age`, `highlight_match`, `read_stdin_text`, `sanitize_display`)

`src/main.rs` calls into `src/lib.rs` only. Keep terminal concerns in the binary and core logic in the library.

**Terminal-injection guard:** any note text or tag that reaches a `println!`/`eprint!` must go through `sanitize_display` (`src/format.rs`) first — it strips control characters (keeping `\n`/`\t`) so imported or piped note content can't inject ANSI/CSI escape sequences into the user's terminal. Every `cmd_*` in `main.rs` already routes through it via the `display_note`/`display_tags` helpers; follow the same pattern for any new output.

**CLI error convention:** every `cmd_*` function returns `Result<(), String>`; `main()` matches on the command, and any `Err` bubbles up to a single site that prints `Error: {msg}` and exits 1 (`fatal`, `src/main.rs`), first zeroing `ACTIVE_PASSWORD` if it was set (so a failed command never leaves a password sitting in memory for a later, unrelated call within the same process — relevant to library consumers more than the one-shot CLI).

**Known rough edge, verified:** `text_from_stdin_or_args` (`src/main.rs`) returns whatever error `read_stdin_text` produces, but its three call sites (`add` at `src/main.rs:414`, `edit` at `:443`, `append` at `:491`) all do `.unwrap_or_else(|_| fatal("usage: scriv <cmd> ..."))`. So if you pipe more than 10 MB into `scriv add`/`edit`/`append`, the real error ("stdin input exceeds 10 MB limit") is discarded and replaced with a generic usage message — worth knowing before you go looking for a phantom bug in `read_stdin_text` itself.

### Storage

Notes persist as **NDJSON** (one JSON object per line) at a platform-specific path resolved by `notes_path()` in `src/storage.rs` — despite the file being named `notes.json`, it is not a single JSON document. Default paths (no override active):

| Platform | Path |
|---|---|
| Linux / WSL | `$XDG_DATA_HOME/scriv/notes.json`, falling back to `~/.local/share/scriv/notes.json` |
| macOS | `~/Library/Application Support/scriv/notes.json` |
| Windows | `%APPDATA%\scriv\notes.json` |

`set_notes_path_override(Some(path))` redirects this (used by tests); `None` restores the default. Writes are atomic: `save_notes` builds the full NDJSON in memory, writes it to a `tempfile` in the same directory, then `persist`s it over the real path — a crash mid-write can't corrupt the existing file. On Unix, the data directory gets `0o700` and the temp file `0o600`, but only the first time the directory is created (`dir_existed` check in `src/storage.rs`); a pre-existing directory keeps whatever permissions it already had.

Size limits, all in `src/notes.rs` / `src/format.rs` and enforced before any I/O: `MAX_NOTE_BYTES` = 1 MiB (add/edit/append text), `MAX_IMPORT_BYTES` = 50 MiB (`import`/`parse_import_ndjson`), and the private `MAX_STDIN_BYTES` = 10 MiB (`read_stdin_text`, applies to piped `add`/`edit`/`append`).

Timestamps (`created_at`, `updated_at` on `Note`) are plain RFC3339 `String`s, not `chrono::DateTime` — `chrono` is pulled in with `default-features = false, features = ["clock", "std"]` (`Cargo.toml`), i.e. no `serde` feature, so a typed field wouldn't round-trip through `serde_json` as-is. Anything that needs to compare or format them parses via `chrono::DateTime::parse_from_rfc3339` on demand (see `note_age`, `validate_note`, `cmd_view`).

### Encryption

`src/crypto.rs`: AES-256-GCM, key via PBKDF2-HMAC-SHA256 with 600,000 iterations, 32-byte salt, 12-byte nonce. On-disk layout is `ENCRYPTED_MAGIC (6 bytes, "scriv\x01") || salt(32) || nonce(12) || ciphertext+tag`. `is_encrypted_data`/`notes_file_is_encrypted` decide encrypted-vs-plain purely by checking those first 6 bytes — there is no separate metadata flag, so a store's "locked" state is entirely a property of the file's own contents.

The active password lives in the process-global `ACTIVE_PASSWORD` (`src/storage.rs`), set via `set_active_password`/`set_active_password_zeroized`. It's wrapped in `zeroize::Zeroizing` everywhere it's threaded through (`Zeroizing<String>` in `main.rs`'s `prompt_password`, `Zeroizing<Vec<u8>>` for decrypted buffers in `load_notes`) — any new code that touches password or plaintext bytes should keep using `Zeroizing` rather than a bare `String`/`Vec<u8>`, to stay consistent with the rest of the crate. `active_password()` (plain, non-zeroizing) is `#[deprecated(since = "1.3.0", ...)]` and kept only via `#[allow(deprecated)]` in `src/lib.rs` for semver — don't call it in new code, use `has_active_password()`.

There is no password-recovery path: `decrypt_notes` returns a generic `"incorrect password"` on any wrong password, and forgetting it means the data in that file is unrecoverable by design. This is a distinct failure mode from a corrupted (non-encrypted-header, unparseable) file, which instead points the user at `scriv clear --force` (`src/storage.rs::load_notes`) — don't conflate the two error paths if you touch this code.

`main.rs`'s `NO_PASSWORD_PROMPT_COMMANDS` (`lock`, `unlock`, help/version variants) is the list of commands that skip the upfront password prompt even when the store is encrypted; every other command — including `clear`, `export`, `tags` — prompts first if `notes_file_is_encrypted()` is true.

### ID assignment

IDs are not sequential from a counter — a new note gets `max(existing IDs) + 1` (`next_id`, `src/notes.rs`), saturating into an error ("note id limit reached") rather than wrapping at `u64::MAX`. IDs are stable after deletion (gaps are preserved). `import_notes` always reassigns every incoming note's ID against the current max, even ones that wouldn't have conflicted — it never preserves an imported note's original ID.

`remove_notes(ids, force)`: with `force = false` it's all-or-nothing — if any requested ID doesn't exist, nothing is removed and an error lists the missing IDs; `force = true` removes whatever exists and silently skips the rest. `scriv done --force` maps to the latter.

### Dependency-version gotchas

- `rand = "0.10"`: `src/crypto.rs` uses `rand::rng()` (not the older `thread_rng()`) and imports `rand::RngExt` (line 6) to get `.fill()` — a bare `use rand::Rng` is not enough on this version.
- `aes-gcm = "0.11"` / pinned `hybrid-array 0.4.12`: `Nonce::from_slice` is deprecated on this pin (see the clippy caveat above); it still works, just emits a warning `-D warnings` will catch.

## Backwards compatibility

This crate is published on crates.io and consumed as a library (current version 2.1.0, per `Cargo.toml`). All public API in `src/lib.rs` must follow semver:

- **Patch** (x.y.Z): bug fixes and internal changes only. No changes to public function signatures, return types, or observable behavior (e.g., a function that previously returned `Ok` must not start returning `Err` for the same inputs).
- **Minor** (x.Y.0): new public functions or fields are OK. Existing signatures and behavior must not break.
- **Major** (X.0.0): required for any breaking change to public API (changed return types, removed functions, changed error conditions). This crate has already done one major bump (the `active_password` deprecation dates from 1.3.0, current is 2.x), so treat the policy as live, not theoretical.

`notes.json` is user data that persists across app versions. Never rename or remove existing JSON keys on `Note` (`id`, `text`, `created_at`, `updated_at`, `tags`). New optional fields must use serde defaults/skip-serialization behavior to preserve compatibility.

## Stale/incomplete existing docs

- **`README.md`** documents every CLI command *except* `lock`/`unlock`, even though password protection is the crate's headline secondary feature (Cargo.toml description: "...with optional password encryption") and both commands are fully implemented (`cmd_lock`/`cmd_unlock`, `src/main.rs`). Not wrong, just incomplete — don't assume README's command list is exhaustive when reasoning about CLI surface; check `main.rs`'s match arms instead. Flagging rather than editing README, since that wasn't asked for here.
