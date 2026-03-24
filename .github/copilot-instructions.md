# Project Guidelines

## Build and Test
Use these commands when validating changes:

```sh
go build .
go test ./...
go vet ./...
```

Use targeted tests while iterating, then run `go test ./...` before finishing.

## Architecture
This is a single-package Go CLI (`package main`) with clear file boundaries:

- `main.go`: CLI entry point and command dispatch.
- `commands.go`: CLI-facing handlers (`cmd*`) for input/output and prompts.
- `notes.go`: data/business layer (`Note`, persistence, search, add/remove, crypto helpers).

Keep business logic in `notes.go` and keep terminal/user interaction in `commands.go`.

## Conventions
- Treat notes JSON as user data with backwards compatibility guarantees.
- Do not rename or remove existing JSON keys on `Note`.
- New optional `Note` fields must use `omitempty`.
- Preserve ID behavior: new IDs are `max(existing)+1` and gaps remain after deletions.
- In tests, isolate storage with `setupTempFile(t)` so real user notes are never touched.
- `loadNotes()` should continue treating a missing notes file as an empty dataset, not an error.

## Docs
Link to existing docs instead of duplicating details:

- `README.md` for usage and platform install/storage details.
- `CLAUDE.md` for concise maintainer expectations and architecture notes.
