# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
go build .          # build binary
go install .        # build and install to $GOPATH/bin
go test ./...       # run all tests
go test -run TestAddNote_AssignsID1WhenEmpty  # run a single test
go vet ./...        # lint
```

## Architecture

`jot` is a single-package CLI (`package main`) with no external dependencies. Three files divide responsibilities:

- **`main.go`** — entry point, command dispatch via `os.Args` switch
- **`commands.go`** — `cmd*` functions that handle CLI I/O (print output, prompt user)
- **`notes.go`** — pure data layer: `Note` struct, `loadNotes`/`saveNotes`, and business logic (`addNote`, `removeNote`, `searchNotes`, `clearNotes`)

The `commands.go` functions call into `notes.go` functions. The data layer has no knowledge of CLI output.

### Storage

Notes persist as JSON at a platform-specific path resolved by `notesPath()` in `notes.go`. Tests override this via the package-level `notesPathOverride` variable — set it in test setup via `setupTempFile(t)` to isolate tests from the real notes file.

### ID assignment

IDs are not sequential integers from a counter — new notes get `max(existing IDs) + 1`. IDs are stable after deletion (gaps are preserved).

### Backwards compatibility

`notes.json` is user data that persists across app versions. Never rename or remove existing JSON keys on the `Note` struct. New optional fields must use `omitempty`. The `notes_compat_test.go` file pins the current schema — all fixtures there must continue to load correctly.
