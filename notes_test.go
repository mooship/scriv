package main

import (
	"os"
	"path/filepath"
	"testing"
)

func setupTempFile(t *testing.T) func() {
	t.Helper()
	dir := t.TempDir()
	notesPathOverride = filepath.Join(dir, "notes.json")
	return func() { notesPathOverride = "" }
}

func TestLoadNotes_MissingFileReturnsEmpty(t *testing.T) {
	defer setupTempFile(t)()

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 0 {
		t.Errorf("expected empty slice, got %d notes", len(notes))
	}
}

func TestLoadNotes_CorruptedFileReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	os.WriteFile(notesPathOverride, []byte("not json"), 0644)

	_, err := loadNotes()
	if err == nil {
		t.Fatal("expected error for corrupted file, got nil")
	}
}

func TestLoadNotes_ValidFile(t *testing.T) {
	defer setupTempFile(t)()

	json := `[{"id":1,"text":"hello","created_at":"2025-01-01T00:00:00Z"}]`
	os.WriteFile(notesPathOverride, []byte(json), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 1 || notes[0].ID != 1 || notes[0].Text != "hello" {
		t.Errorf("unexpected notes: %+v", notes)
	}
}

func TestAddNote_AssignsID1WhenEmpty(t *testing.T) {
	defer setupTempFile(t)()

	note, err := addNote("first")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.ID != 1 {
		t.Errorf("expected ID 1, got %d", note.ID)
	}
}

func TestAddNote_IDIsMaxPlusOne(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")
	note, err := addNote("three")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.ID != 3 {
		t.Errorf("expected ID 3, got %d", note.ID)
	}
}

func TestAddNote_PersistsToDisk(t *testing.T) {
	defer setupTempFile(t)()

	addNote("persisted")

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 1 || notes[0].Text != "persisted" {
		t.Errorf("unexpected notes: %+v", notes)
	}
}

func TestAddNote_SetsCreatedAt(t *testing.T) {
	defer setupTempFile(t)()

	note, _ := addNote("timestamped")
	if note.CreatedAt == "" {
		t.Error("expected CreatedAt to be set")
	}
}

func TestRemoveNote_RemovesCorrectNote(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")
	addNote("gamma")

	removed, err := removeNote(2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if removed.Text != "beta" {
		t.Errorf("expected to remove beta, got %s", removed.Text)
	}
}

func TestRemoveNote_LeavesOthersIntact(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")
	addNote("gamma")
	removeNote(2)

	notes, _ := loadNotes()
	if len(notes) != 2 {
		t.Fatalf("expected 2 notes, got %d", len(notes))
	}
	if notes[0].ID != 1 || notes[1].ID != 3 {
		t.Errorf("unexpected IDs: %d, %d", notes[0].ID, notes[1].ID)
	}
}

func TestRemoveNote_IDsAreStable(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")
	addNote("three")
	removeNote(2)

	note, _ := addNote("four")
	if note.ID != 4 {
		t.Errorf("expected new note to get ID 4, got %d", note.ID)
	}
}

func TestRemoveNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")

	_, err := removeNote(99)
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestSearchNotes_CaseInsensitive(t *testing.T) {
	defer setupTempFile(t)()

	addNote("Fix the Auth Bug")
	addNote("write tests")

	results, err := searchNotes("auth")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 || results[0].ID != 1 {
		t.Errorf("unexpected results: %+v", results)
	}
}

func TestSearchNotes_NoMatches(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")

	results, err := searchNotes("zzz")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 0 {
		t.Errorf("expected no results, got %d", len(results))
	}
}

func TestSearchNotes_MultipleMatches(t *testing.T) {
	defer setupTempFile(t)()

	addNote("fix bug one")
	addNote("fix bug two")
	addNote("unrelated")

	results, err := searchNotes("fix")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 2 {
		t.Errorf("expected 2 results, got %d", len(results))
	}
}

func TestClearNotes_EmptiesFile(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")

	if err := clearNotes(); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	notes, _ := loadNotes()
	if len(notes) != 0 {
		t.Errorf("expected empty notes after clear, got %d", len(notes))
	}
}

func TestCmdDone_InvalidIDString(t *testing.T) {
	defer setupTempFile(t)()

	err := cmdDone("abc")
	if err == nil {
		t.Fatal("expected error for non-integer ID")
	}
}

func TestCmdDone_ZeroIDIsInvalid(t *testing.T) {
	defer setupTempFile(t)()

	err := cmdDone("0")
	if err == nil {
		t.Fatal("expected error for ID 0")
	}
}

func TestCmdDone_ValidIDRemovesNote(t *testing.T) {
	defer setupTempFile(t)()

	addNote("to remove")

	if err := cmdDone("1"); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	notes, _ := loadNotes()
	if len(notes) != 0 {
		t.Error("expected note to be removed")
	}
}
