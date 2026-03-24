# Project Guidelines

## Build and Test
Use these commands when validating changes:

```sh
cargo build
cargo test
cargo clippy -- -D warnings
```

Use targeted tests while iterating, then run `cargo test` before finishing.

## Architecture
This is a Rust CLI crate with binary + library split:

- `src/main.rs`: CLI entry point, command dispatch, and terminal I/O.
- `src/lib.rs`: public API and re-exports.
- `src/model.rs`: core data types (`Note`, `ListOptions`).
- `src/storage.rs`: persistence, notes path resolution, and active password state.
- `src/crypto.rs`: encryption and decryption helpers.
- `src/notes.rs`: business operations (add/remove/edit/search/tag/list).
- `src/format.rs`: input/output formatting helpers.

Keep business logic in module files under `src/` and keep terminal/user interaction in `src/main.rs`.

## Conventions
- Treat notes JSON as user data with backwards compatibility guarantees.
- Do not rename or remove existing JSON keys on `Note`.
- New optional `Note` fields must use serde defaults and skip-serialization behavior.
- Preserve ID behavior: new IDs are `max(existing)+1` and gaps remain after deletions.
- In tests, isolate storage with `set_notes_path_override(...)` so real user notes are never touched.
- `load_notes()` should continue treating a missing notes file as an empty dataset, not an error.

## Docs
Link to existing docs instead of duplicating details:

- `README.md` for usage and platform install/storage details.
- `CLAUDE.md` for concise maintainer expectations and architecture notes.
