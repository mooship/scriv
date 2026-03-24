# jot

Fast, local CLI note manager. Notes are stored as local NDJSON (or encrypted with a password). No daemon, no sync, no accounts.

## Install

Prerequisite: install Rust via rustup.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install from crates.io:

```sh
cargo install jot
```

Install from this repository (no local clone required):

```sh
cargo install --git https://github.com/mooship/jot
```

Or clone and install locally:

```sh
git clone https://github.com/mooship/jot
cd jot
cargo install --path .
```

`cargo install` places `jot` in Cargo's bin directory (`$HOME/.cargo/bin` on Unix, `%USERPROFILE%\.cargo\bin` on Windows). Make sure that path is on your `PATH`.

### crates.io

Crate page: https://crates.io/crates/jot

### Build from source

```sh
cargo build --release
```

## Usage

```sh
# Add a note
jot add "fix the auth bug"
# Added [1] fix the auth bug

# Pipe text in from stdin
echo "buy oat milk" | jot add

# List all notes (shows age)
jot list
# [1] (2d) fix the auth bug
# [2] (1h) write tests
# [3] (<1h) update README
# 3 notes.

# Limit to the 5 most recent
jot list --limit=5

# Filter by tag (case-insensitive)
jot list --tag=work

# Sort by last-updated
jot list --sort=updated

# Show full text without truncation
jot list --full

# Edit a note
jot edit 1 "fix the auth bug (critical)"
# Updated [1] fix the auth bug (critical)

# Edit via stdin
echo "fix the auth bug (done)" | jot edit 1

# Append to a note
jot append 1 "— assigned to alice"
# Updated [1] fix the auth bug (critical) — assigned to alice

# Mark one or more notes done (removes them)
jot done 2
# Removed [2] write tests

jot done 1 3
# Removed [1] fix the auth bug (critical) — assigned to alice
# Removed [3] update README

# Skip missing IDs instead of erroring (useful in scripts)
jot done --force 1 99

# Search notes (text and tags); matches are highlighted in the terminal
jot search auth
# [1] fix the auth bug
# 1 matches.

# View full details of a note
jot view 1
# [1] fix the auth bug
#     Created: 2026-03-12
#     Updated: 2026-03-14
#     Tags: #work #critical

# Tag a note
jot tag 1 work critical
# Tagged [1] fix the auth bug: #work #critical

# Remove a tag
jot untag 1 critical
# Removed tag #critical from [1] fix the auth bug

# List all tags with note counts
jot tags
# critical             1
# work                 3

# Clear all notes (prompts for confirmation)
jot clear
# Remove all 2 notes? [y/N] y
# Cleared.

# Clear without prompt
jot clear --force

# Export all notes as NDJSON
jot export > backup.ndjson

# Import notes from NDJSON (IDs are reassigned to avoid conflicts)
jot import < backup.ndjson
# Imported 3 notes.
```

## Storage

Notes are saved to a local NDJSON file (or encrypted bytes when locked) - nothing leaves your machine.

| Platform | Path |
|---|---|
| Linux / WSL | `~/.local/share/jot/notes.json` |
| macOS | `~/Library/Application Support/jot/notes.json` |
| Windows | `%APPDATA%\jot\notes.json` |

The file is created automatically on first use.

## License

[GNU General Public License v3.0](LICENSE)
