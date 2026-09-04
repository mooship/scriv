# scriv

Fast, local CLI note manager. Notes are stored as local NDJSON (or encrypted with a password). No daemon, no sync, no accounts.

## Install

Prerequisite: install Rust via rustup.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install from crates.io:

```sh
cargo install scriv
```

Install from this repository (no local clone required):

```sh
cargo install --git https://github.com/mooship/scriv
```

Or clone and install locally:

```sh
git clone https://github.com/mooship/scriv
cd scriv
cargo install --path .
```

`cargo install` places `scriv` in Cargo's bin directory (`$HOME/.cargo/bin` on Unix, `%USERPROFILE%\.cargo\bin` on Windows). Make sure that path is on your `PATH`.

### crates.io

Crate page: https://crates.io/crates/scriv

### Build from source

```sh
cargo build --release
```

## Usage

```sh
# Add a note
scriv add "fix the auth bug"
# Added [1] fix the auth bug

# Pipe text in from stdin
echo "buy oat milk" | scriv add

# List all notes (shows age)
scriv list
# [1] (2d) fix the auth bug
# [2] (1h) write tests
# [3] (<1h) update README
# 3 notes.

# Limit to the 5 most recent
scriv list --limit=5

# Filter by tag (case-insensitive)
scriv list --tag=work

# Sort by last-updated
scriv list --sort=updated

# Show full text without truncation
scriv list --full

# Edit a note
scriv edit 1 "fix the auth bug (critical)"
# Updated [1] fix the auth bug (critical)

# Edit via stdin
echo "fix the auth bug (done)" | scriv edit 1

# Append to a note
scriv append 1 "— assigned to alice"
# Updated [1] fix the auth bug (critical) — assigned to alice

# Mark one or more notes done (removes them)
scriv done 2
# Removed [2] write tests

scriv done 1 3
# Removed [1] fix the auth bug (critical) — assigned to alice
# Removed [3] update README

# Skip missing IDs instead of erroring (useful in scripts)
scriv done --force 1 99

# Search notes (text and tags); matches are highlighted in the terminal
scriv search auth
# [1] fix the auth bug
# 1 matches.

# View full details of a note
scriv view 1
# [1] fix the auth bug
#     Created: 2026-03-12
#     Updated: 2026-03-14
#     Tags: #work #critical

# Tag a note
scriv tag 1 work critical
# Tagged [1] fix the auth bug: #work #critical

# Remove a tag
scriv untag 1 critical
# Removed tag #critical from [1] fix the auth bug

# List all tags with note counts
scriv tags
# critical             1
# work                 3

# Clear all notes (prompts for confirmation)
scriv clear
# Remove all 2 notes? [y/N] y
# Cleared.

# Clear without prompt
scriv clear --force

# Export all notes as NDJSON
scriv export > backup.ndjson

# Import notes from NDJSON (IDs are reassigned to avoid conflicts)
scriv import < backup.ndjson
# Imported 3 notes.

# Password-protect your notes (prompts for a new password twice)
scriv lock
# New password:
# Confirm password:
# Notes are now password protected.

# Once locked, every other command prompts for the password first
scriv list
# Password:
# [1] (2d) fix the auth bug
# 1 notes.

# Change the password (prompts for the current one first, then a new one)
scriv lock
# Current password:
# New password:
# Confirm password:
# Notes are now password protected.

# Remove password protection (prompts for the current password)
scriv unlock
# Password:
# Password protection removed.
```

## Password Protection

`scriv lock` sets or changes the password used to encrypt your notes file. If
the file is not yet encrypted, it only prompts for a new password (entered
twice, to confirm). If it's already encrypted, it first prompts for the
current password to decrypt the existing notes before re-encrypting them
under the new one. An empty new password, or a confirmation that doesn't
match, is rejected without changing anything.

`scriv unlock` removes password protection, decrypting the notes file and
saving it as plain NDJSON. If the notes aren't currently encrypted, it prints
a message and does nothing.

Once a notes file is encrypted, every other command — `add`, `list`, `edit`,
`clear`, `export`, and so on — prompts for the password before it can read or
write the file. Only `lock`, `unlock`, `--help`, and `--version` skip that
prompt.

There is no password recovery. If you forget your password, the notes in
that file cannot be decrypted — keep a plaintext export (`scriv export`)
somewhere safe if that risk matters to you, and take it *before* locking.

## Shell Alias

If you'd like a shorter command, add an alias to your shell configuration:

**Bash** (`~/.bashrc`):
```sh
alias s='scriv'
```

**Zsh** (`~/.zshrc`):
```sh
alias s='scriv'
```

**Fish** (`~/.config/fish/config.fish`):
```fish
alias s 'scriv'
```

**PowerShell** (`$PROFILE`):
```powershell
Set-Alias -Name s -Value scriv
```

Reload your shell (or `source` the file) and use `s` in place of `scriv`:

```sh
s add "remember to hydrate"
s list
s done 1
```

## Storage

Notes are saved to a local NDJSON file (one JSON object per line), or as
encrypted bytes when locked — nothing leaves your machine.

| Platform | Path |
|---|---|
| Linux / WSL | `$XDG_DATA_HOME/scriv/notes.json`, falling back to `~/.local/share/scriv/notes.json` |
| macOS | `~/Library/Application Support/scriv/notes.json` |
| Windows | `%APPDATA%\scriv\notes.json` |

The file is created automatically on first use. Writes are atomic (via a
temporary file in the same directory), so an interrupted write can't corrupt
existing notes.

### Limits

- A single note's text: 1 MiB
- Piped stdin input (`add`/`edit`/`append`): 10 MB
- `import` input: 50 MB

## License

[GNU General Public License v3.0](LICENSE)
