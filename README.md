# jot

Fast, local CLI note manager. Notes are stored as plain JSON on disk. No daemon, no sync, no accounts.

## Install

```sh
go install github.com/mooship/jot@latest
```

This builds the binary and places it in `$GOPATH/bin` (usually `~/go/bin`).

Make sure that directory is on your PATH:

**Linux / WSL** — add to `~/.bashrc` or `~/.zshrc`:
```sh
export PATH="$PATH:$(go env GOPATH)/bin"
```
Then run `source ~/.bashrc`.

**macOS** — add to `~/.zshrc` (or `~/.bash_profile` if using bash):
```sh
export PATH="$PATH:$(go env GOPATH)/bin"
```
Then run `source ~/.zshrc`.

**Windows** — run this in PowerShell (once):
```powershell
$gobin = "$(go env GOPATH)\bin"
[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";$gobin", "User")
```
Then restart your terminal.

### Build from source

```sh
git clone https://github.com/mooship/jot
cd jot
go install .
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

# Filter by tag
jot list --tag=work

# Sort by last-updated
jot list --sort=updated

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

# Search notes (text and tags)
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
```

## Storage

Notes are saved to a local JSON file — nothing leaves your machine.

| Platform | Path |
|---|---|
| Linux / WSL | `~/.local/share/jot/notes.json` |
| macOS | `~/Library/Application Support/jot/notes.json` |
| Windows | `%APPDATA%\jot\notes.json` |

The file is created automatically on first use.
