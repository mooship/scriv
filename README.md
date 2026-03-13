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

# List all notes
jot list
# [1] fix the auth bug
# [2] write tests
# [3] update README

# Edit a note
jot edit 1 "fix the auth bug (critical)"
# Updated [1] fix the auth bug (critical)

# Mark a note done (removes it)
jot done 2
# Removed [2] write tests

# Search notes
jot search auth
# [1] fix the auth bug

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
