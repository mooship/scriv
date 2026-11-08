# AGENTS.md

Guidance for AI coding agents and contributors working in this repository.

## About

Scriv (`Cargo.toml`) is a local CLI note manager, edition 2024, MSRV 1.88, currently at version 2.1.0, licensed GPL-3.0-only (`LICENSE`). Published on crates.io and docs.rs as a library + binary. No network calls, no daemon, no accounts: everything lives in one local file.

The repo is small and flat: `src/` (8 files), `tests/` (6 integration files + `tests/common/`), `.github/` (CI + dependabot), plus `Cargo.toml`/`Cargo.lock`, `README.md`, `LICENSE`, `.gitattributes`, `.gitignore`. No `docs/`, no `rust-toolchain.toml`, no `rustfmt.toml`/`clippy.toml` (both tools run on their defaults), no dev-dependencies section: `tempfile` and `once_cell` are regular dependencies (used by `src/storage.rs`) that the test helpers in `tests/common/mod.rs` reuse. The only profile tweak is `[profile.dev.package."*"] opt-level = 3` in `Cargo.toml`, which keeps the 600,000 PBKDF2 rounds from taking seconds per call in tests.

## Commands

```sh
cargo build              # build binary
cargo install --path .   # build and install to Cargo bin dir
cargo test               # run all tests (integration only, no #[cfg(test)] units in src/)
cargo test add_note_assigns_id_1_when_empty  # run a single test
cargo fmt                # format code (default rustfmt settings)
cargo clippy --all-targets -- -D warnings    # lint, tests included
cargo llvm-cov --fail-under-lines 95 --fail-under-functions 95 --fail-under-regions 95   # coverage gate
```

`cargo llvm-cov` needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`. Add `--show-missing-lines` to list uncovered lines.

CI (`.github/workflows/ci.yml`) runs on push to `main`, on every PR, and weekly (Saturday 07:00 UTC), on `ubuntu-latest` only:

- Job `build-and-test`: `cargo fmt -- --check`, then `cargo clippy --locked --all-targets -- -D warnings` (piped through `clippy-sarif`/`sarif-fmt` and uploaded to GitHub code scanning), `cargo build --locked`, `cargo test --locked`. The Clippy step sets `shell: bash` so `pipefail` applies: a clippy failure fails the job, and the SARIF upload still runs (`if: always()`). Don't drop `shell: bash`; without it only `sarif-fmt`'s exit status counts and clippy can fail silently.
- Job `coverage`: `cargo llvm-cov --locked` with the 95% line, function and region gate. The job fails below the gate.

There's no `[features]` table in `Cargo.toml`, so `--all-features` would be a no-op. The Windows and macOS branches of `notes_path()` (`src/storage.rs`) are never exercised by CI, only by reading the code.

## Engineering principles

- **Tests first.** For logic and bug fixes, write the failing test in `tests/` before the code, watch it fail for the right reason, make it pass, then tidy. Library behaviour goes in the matching `*_tests.rs`; anything the binary does (flag parsing, exit codes, what reaches stdout and stderr) goes in `tests/cli_tests.rs`, which runs the built binary. Interactive password prompts can't be driven in tests (`rpassword` reads `/dev/tty`), which is why `lock_notes`/`unlock_notes` take the prompt as a closure. Never weaken, skip or delete a test to get green, and never add a coverage-ignore attribute to move the number: the 95% gate is a floor and 100% is the aim.
- **One module, one reason to change.** `model` holds the data types, `storage` the path, persistence and active password, `crypto` the cipher, `notes` the note operations, `format` the display and search helpers, `lock` the password lifecycle, and `main.rs` only terminal I/O. The binary calls the library's public API and nothing else.
- **Open for extension, closed for edits.** New behaviour is a new function in the module that owns that concern, re-exported from `src/lib.rs`, plus a new `cmd_*` and match arm in `main.rs`; it should not need edits scattered across existing modules. The seams that already exist are the closure that `modify_note` takes, the `Read` generics on `parse_import_ndjson` and `read_stdin_text`, and the prompt closure on `lock_notes`/`unlock_notes`. Don't invent more. Keep the public API narrow: the crypto re-exports are `#[doc(hidden)]` and existing signatures follow the semver rules below.
- **DRY.** Send every user-facing note string through `sanitize_display` (via `display_note`/`display_tags`) and thread secrets through `Zeroizing`. Add, edit and import all validate through `validate_text`/`validate_note`, and the usage text lives once in `USAGE_TEMPLATE`. Don't merge code that only looks alike: `load_notes`, `parse_import_ndjson` and `read_stdin_text` all read lines but enforce different limits and report different errors.
- **KISS and YAGNI.** This is a small, flat, single-user CLI with no daemon and no network calls. Prefer a plain function in the matching `src/*.rs` file over a new abstraction layer, and don't add config surface or feature flags for a use case nobody has asked for.
- **Stay on task.** Don't refactor what the task doesn't touch; mention nearby problems instead.

## Conventions

### Commit messages

Conventional Commits 1.0.0: `type(scope): description` with a lowercase type, and a body that explains why. No em dashes or en dashes.

### Code style

- **No trailing comments**: never put a `//` comment on the same line as code. Use `///` doc comments where genuinely useful, and a `//` comment on its own line only for a non-obvious why (for example a `SAFETY:` note).
- **British English spelling** in user-facing copy (CLI help text, error messages, `README.md`): "colour", "organise". Code identifiers stay standard English.
- **No em dashes** in code, comments, docs or commit messages.
- Run `cargo fmt` before committing. There's no `rustfmt.toml`, so behaviour can shift slightly with the rustfmt version (CI floats `dtolnay/rust-toolchain@stable`, there's no pinned toolchain file).
- `cargo clippy --all-targets -- -D warnings` must pass, and CI enforces it.

## Definition of done

Run these before you call a change finished, and say so in your hand-off if any were skipped (and why):

1. `cargo fmt -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo llvm-cov --fail-under-lines 95 --fail-under-functions 95 --fail-under-regions 95`

Behaviour changes need a test that fails without them. A change to the CLI surface or storage format also needs `README.md` updated. Don't bump the crate version unless releasing.

## Safety

- **Never publish (`cargo publish`) or run destructive storage operations without explicit permission from the user.** Always ask first and wait for confirmation. Destructive means anything that rewrites or removes a real notes file, such as `scriv clear`, `scriv lock` or `scriv unlock` against the user's own data directory. Tests must only touch temp directories (the `TestEnv` path override, or the per-test data directory in `tests/cli_tests.rs`).
- Never put a real password or note content in a commit, test fixture or log.

## Testing

Tests are integration tests under `tests/`, sharing helpers from `tests/common/mod.rs`. Run all with `cargo test`.

- `crypto_tests.rs`: the encrypted file layout (checked against an independent AES-GCM/PBKDF2 implementation in both directions), wrong password, tampering.
- `format_tests.rs`: pure display and stdin helpers.
- `lib_tests.rs`: note operations, validation, import parsing and compatibility fixtures.
- `lock_tests.rs`: `lock_notes`/`unlock_notes` with scripted prompts.
- `storage_tests.rs`: persistence, error paths and file permissions.
- `cli_tests.rs`: runs the built binary (`CARGO_BIN_EXE_scriv`) against a throwaway data directory.

`src/storage.rs` holds process-global mutable state: `NOTES_PATH_OVERRIDE` and `ACTIVE_PASSWORD` (both `Lazy<Mutex<_>>`). `cargo test` runs tests within one binary in parallel threads by default, and all tests in `lib_tests.rs`, `lock_tests.rs` and `storage_tests.rs` share that same process-wide state. That's why every such test starts with:

```rust
let _guard = lock_test();   // tests/common/mod.rs, global Mutex, serialises these tests
let _env = TestEnv::new();  // redirects notes_path() to a temp file, clears the active password
```

Any new test that touches storage or the active password **must** take `lock_test()` first, or it will race with other tests in the same binary and get spurious failures or cross-test data leakage. `TestEnv::new()`/`Drop` reset both statics; the `_guard` must outlive `_env` (declare it first, as above) since dropping the lock before storage is reset re-opens the race window for the next test.

`cli_tests.rs` is different: every test runs the binary as its own process with `HOME`, `XDG_DATA_HOME` and `APPDATA` pointed at its own temp directory, so it needs neither `lock_test()` nor `TestEnv`. Three things to know when adding CLI tests:

- When stdin isn't a terminal the binary reads its text from stdin and ignores positional text arguments, so tests pass note text on stdin. The positional-argument branch of `text_from_stdin_or_args` only runs on a real terminal and isn't covered.
- On Unix the child is started in its own session (`setsid`), so opening `/dev/tty` fails and a password prompt errors out instead of reading the developer's real terminal. Don't remove this.
- `cmd_search` only highlights matches when stdout is a terminal, so piped tests see plain text. The `Ok` arm of the up-front password prompt in `main` is likewise terminal-only.

## Architecture

`scriv` is a Rust crate with a binary + library split. `[[bin]]`/`[lib]` aren't declared explicitly in `Cargo.toml`; having both `src/main.rs` and `src/lib.rs` is what gives Cargo the two implicit targets, both named after the package.

- **`src/main.rs`** - CLI entry point, arg parsing, terminal I/O, command dispatch (`cmd_*` functions)
- **`src/lib.rs`** - crate API and re-exports; the `crypto` re-exports are `#[doc(hidden)]` (internal use by the binary and tests, not part of the intended public surface)
- **`src/model.rs`** - `Note` and `ListOptions`
- **`src/storage.rs`** - notes path resolution, persistence, active password state
- **`src/crypto.rs`** - AES-256-GCM encryption/decryption
- **`src/lock.rs`** - `lock_notes`/`unlock_notes`: set, change and remove the password, with the prompt passed in as a closure
- **`src/notes.rs`** - core note operations (`add_note`, `remove_notes`, `search_notes`, `clear_notes`, etc.)
- **`src/format.rs`** - display/search helpers (`note_age`, `highlight_match`, `read_stdin_text`, `sanitize_display`)

`src/main.rs` calls into `src/lib.rs` only. Keep terminal concerns in the binary and core logic in the library.

**Terminal-injection guard:** any note text or tag that reaches a `println!`/`eprint!` must go through `sanitize_display` (`src/format.rs`) first. It strips control characters (keeping `\n`/`\t`) so imported or piped note content can't inject ANSI/CSI escape sequences into the user's terminal. Every `cmd_*` in `main.rs` already routes through it via the `display_note`/`display_tags` helpers; follow the same pattern for any new output.

**CLI error convention:** every `cmd_*` function returns `Result<(), String>`. `main()` matches on the command, and any `Err` bubbles up to a single site at the end of `main` that zeroes `ACTIVE_PASSWORD` if it was set (so a failed command never leaves a password sitting in memory for a later, unrelated call within the same process, which matters to library consumers more than the one-shot CLI), prints `Error: {msg}` and exits 1. Usage and flag errors go through `fatal` instead, which prints the same `Error:` line and exits immediately without that cleanup.

**Known rough edge, verified:** `text_from_stdin_or_args` (`src/main.rs`) returns whatever error `read_stdin_text` produces, but its three call sites (`add`, `edit` and `append` in `main`) all do `.unwrap_or_else(|_| fatal("usage: scriv <cmd> ..."))`. So if you pipe more than 10 MB into `scriv add`/`edit`/`append`, the real error ("stdin input exceeds 10 MB limit") is discarded and replaced with a generic usage message. Worth knowing before you go looking for a phantom bug in `read_stdin_text` itself.

### Storage

Notes persist as **NDJSON** (one JSON object per line) at a platform-specific path resolved by `notes_path()` in `src/storage.rs`. Despite the file being named `notes.json`, it is not a single JSON document. Default paths (no override active):

| Platform | Path |
|---|---|
| Linux / WSL | `$XDG_DATA_HOME/scriv/notes.json`, falling back to `~/.local/share/scriv/notes.json` |
| macOS | `~/Library/Application Support/scriv/notes.json` |
| Windows | `%APPDATA%\scriv\notes.json` |

`set_notes_path_override(Some(path))` redirects this (used by tests); `None` restores the default. Writes are atomic: `save_notes` builds the full NDJSON in memory, writes it to a `tempfile` in the same directory, then `persist`s it over the real path, so a crash mid-write can't corrupt the existing file. On Unix, the data directory gets `0o700` and the temp file `0o600`, but only the first time the directory is created (`dir_existed` check in `src/storage.rs`); a pre-existing directory keeps whatever permissions it already had.

Size limits, all in `src/notes.rs` / `src/format.rs` and enforced before any I/O: `MAX_NOTE_BYTES` = 1 MiB (add/edit/append text), `MAX_IMPORT_BYTES` = 50 MiB (`import`/`parse_import_ndjson`), and the private `MAX_STDIN_BYTES` = 10 MiB (`read_stdin_text`, applies to piped `add`/`edit`/`append`).

Timestamps (`created_at`, `updated_at` on `Note`) are plain RFC3339 `String`s, not `chrono::DateTime`. `chrono` is pulled in with `default-features = false, features = ["clock", "std"]` (`Cargo.toml`), i.e. no `serde` feature, so a typed field wouldn't round-trip through `serde_json` as-is. Anything that needs to compare or format them parses via `chrono::DateTime::parse_from_rfc3339` on demand (see `note_age`, `validate_note`, `cmd_view`).

### Encryption

`src/crypto.rs`: AES-256-GCM, key via PBKDF2-HMAC-SHA256 with 600,000 iterations, 32-byte salt, 12-byte nonce. On-disk layout is `ENCRYPTED_MAGIC (6 bytes, "scriv\x01") || salt(32) || nonce(12) || ciphertext+tag`. `is_encrypted_data`/`notes_file_is_encrypted` decide encrypted-vs-plain purely by checking those first 6 bytes. There is no separate metadata flag, so a store's "locked" state is entirely a property of the file's own contents.

The active password lives in the process-global `ACTIVE_PASSWORD` (`src/storage.rs`), set via `set_active_password`/`set_active_password_zeroized`. It's wrapped in `zeroize::Zeroizing` everywhere it's threaded through (`Zeroizing<String>` in `main.rs`'s `prompt_password` and in `src/lock.rs`, `Zeroizing<Vec<u8>>` for decrypted buffers in `load_notes`). Any new code that touches password or plaintext bytes should keep using `Zeroizing` rather than a bare `String`/`Vec<u8>`, to stay consistent with the rest of the crate. `active_password()` (plain, non-zeroizing) is `#[deprecated(since = "1.3.0", ...)]` and kept only via `#[allow(deprecated)]` in `src/lib.rs` for semver. Don't call it in new code, use `has_active_password()`.

There is no password-recovery path: `decrypt_notes` returns a generic `"incorrect password"` on any authentication failure, and forgetting the password means the data in that file is unrecoverable by design. This is a distinct failure mode from a corrupted (non-encrypted-header, unparseable) file, which instead points the user at `scriv clear --force` (`src/storage.rs::load_notes`). Don't conflate the two error paths if you touch this code.

`main.rs`'s `NO_PASSWORD_PROMPT_COMMANDS` (`lock`, `unlock`, help/version variants) is the list of commands that skip the upfront password prompt even when the store is encrypted; every other command, including `clear`, `export` and `tags`, prompts first if `notes_file_is_encrypted()` is true. `lock` and `unlock` do their own prompting inside `lock_notes`/`unlock_notes`.

### ID assignment

IDs are not sequential from a counter: a new note gets `max(existing IDs) + 1` (`next_id`, `src/notes.rs`), saturating into an error ("note id limit reached") rather than wrapping at `u64::MAX`. IDs are stable after deletion (gaps are preserved). `import_notes` always reassigns every incoming note's ID against the current max, even ones that wouldn't have conflicted; it never preserves an imported note's original ID.

`remove_notes(ids, force)`: with `force = false` it's all-or-nothing. If any requested ID doesn't exist, nothing is removed and an error lists the missing IDs; `force = true` removes whatever exists and silently skips the rest. `scriv done --force` maps to the latter.

### Dependency-version gotchas

- `rand = "0.10"`: `src/crypto.rs` uses `rand::rng()` (not the older `thread_rng()`) and imports `rand::RngExt` to get `.fill()`. A bare `use rand::Rng` is not enough on this version.
- `aes-gcm = "0.11"` (locked at 0.11.1) with `hybrid-array 0.4.12`: `Nonce::from_slice` is deprecated. `src/crypto.rs` builds the encryption nonce with `Nonce::<Aes256Gcm>::from([u8; NONCE_LEN])`, which stops compiling if the length ever stops matching the cipher, and reads the stored nonce out of the file with `split_first_chunk::<NONCE_LEN>()`, so no conversion there can fail. Keep it that way rather than reintroducing `from_slice` behind `#[allow(deprecated)]`.

## Backwards compatibility

This crate is published on crates.io and consumed as a library (current version 2.1.0, per `Cargo.toml`). All public API in `src/lib.rs` must follow semver:

- **Patch** (x.y.Z): bug fixes and internal changes only. No changes to public function signatures, return types, or observable behaviour (e.g., a function that previously returned `Ok` must not start returning `Err` for the same inputs).
- **Minor** (x.Y.0): new public functions or fields are OK. Existing signatures and behaviour must not break. `lock_notes` and `unlock_notes` are new public functions since 2.1.0, so the next release needs at least a minor bump.
- **Major** (X.0.0): required for any breaking change to public API (changed return types, removed functions, changed error conditions). This crate has already done one major bump (the `active_password` deprecation dates from 1.3.0, current is 2.x), so treat the policy as live, not theoretical.

`notes.json` is user data that persists across app versions. Never rename or remove existing JSON keys on `Note` (`id`, `text`, `created_at`, `updated_at`, `tags`). New optional fields must use serde defaults/skip-serialization behaviour to preserve compatibility. The encrypted layout above is part of the same promise: `tests/crypto_tests.rs` pins it.
