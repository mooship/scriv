package main

import (
	"os"
	"testing"
)

const fixtureMinimal = `{"id":1,"text":"hello world","created_at":"2024-01-15T10:30:00Z"}`

const fixtureWithUpdatedAt = `{"id":1,"text":"original","created_at":"2024-01-15T10:30:00Z","updated_at":"2024-02-01T08:00:00Z"}`

const fixtureWithTags = `{"id":1,"text":"buy milk","created_at":"2024-01-15T10:30:00Z","tags":["groceries","errands"]}`

const fixtureAllFields = `{"id":42,"text":"full note","created_at":"2024-03-01T12:00:00Z","updated_at":"2024-03-10T09:00:00Z","tags":["work","important"]}`

const fixtureMultipleNotes = `{"id":1,"text":"first","created_at":"2024-01-01T00:00:00Z"}
{"id":2,"text":"second","created_at":"2024-01-02T00:00:00Z","tags":["a"]}
{"id":5,"text":"fifth","created_at":"2024-01-05T00:00:00Z","updated_at":"2024-01-06T00:00:00Z","tags":["b","c"]}`

const fixtureUnknownFields = `{"id":1,"text":"note","created_at":"2024-01-01T00:00:00Z","future_field":"ignored","another":123}`

func TestCompat_MinimalNote_LoadsWithoutError(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureMinimal), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 1 {
		t.Fatalf("expected 1 note, got %d", len(notes))
	}
}

func TestCompat_MinimalNote_FieldValues(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureMinimal), 0644)

	notes, _ := loadNotes()
	n := notes[0]
	if n.ID != 1 {
		t.Errorf("expected ID 1, got %d", n.ID)
	}
	if n.Text != "hello world" {
		t.Errorf("expected text 'hello world', got %q", n.Text)
	}
	if n.CreatedAt != "2024-01-15T10:30:00Z" {
		t.Errorf("expected created_at '2024-01-15T10:30:00Z', got %q", n.CreatedAt)
	}
	if n.UpdatedAt != "" {
		t.Errorf("expected empty updated_at, got %q", n.UpdatedAt)
	}
	if n.Tags != nil {
		t.Errorf("expected nil tags, got %v", n.Tags)
	}
}

func TestCompat_WithUpdatedAt_LoadsWithoutError(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureWithUpdatedAt), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if notes[0].UpdatedAt != "2024-02-01T08:00:00Z" {
		t.Errorf("unexpected updated_at: %q", notes[0].UpdatedAt)
	}
}

func TestCompat_WithTags_LoadsWithoutError(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureWithTags), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes[0].Tags) != 2 {
		t.Fatalf("expected 2 tags, got %v", notes[0].Tags)
	}
	if notes[0].Tags[0] != "groceries" || notes[0].Tags[1] != "errands" {
		t.Errorf("unexpected tags: %v", notes[0].Tags)
	}
}

func TestCompat_AllFields_LoadsCorrectly(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureAllFields), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	n := notes[0]
	if n.ID != 42 {
		t.Errorf("expected ID 42, got %d", n.ID)
	}
	if n.Text != "full note" {
		t.Errorf("expected 'full note', got %q", n.Text)
	}
	if n.CreatedAt != "2024-03-01T12:00:00Z" {
		t.Errorf("unexpected created_at: %q", n.CreatedAt)
	}
	if n.UpdatedAt != "2024-03-10T09:00:00Z" {
		t.Errorf("unexpected updated_at: %q", n.UpdatedAt)
	}
	if len(n.Tags) != 2 || n.Tags[0] != "work" || n.Tags[1] != "important" {
		t.Errorf("unexpected tags: %v", n.Tags)
	}
}

func TestCompat_MultipleNotes_PreservesAllNotes(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureMultipleNotes), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 3 {
		t.Fatalf("expected 3 notes, got %d", len(notes))
	}
	if notes[0].ID != 1 || notes[1].ID != 2 || notes[2].ID != 5 {
		t.Errorf("unexpected IDs: %d, %d, %d", notes[0].ID, notes[1].ID, notes[2].ID)
	}
}

func TestCompat_MultipleNotes_IDGapsPreserved(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureMultipleNotes), 0644)

	note, err := addNote("new")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.ID != 6 {
		t.Errorf("expected new note ID 6 (max+1), got %d", note.ID)
	}
}

func TestCompat_UnknownFields_IgnoredOnLoad(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureUnknownFields), 0644)

	notes, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error loading file with unknown fields: %v", err)
	}
	if len(notes) != 1 || notes[0].ID != 1 || notes[0].Text != "note" {
		t.Errorf("unexpected notes: %+v", notes)
	}
}

func TestCompat_JSONKeys_IDField(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(`{"id":7,"text":"x","created_at":"2024-01-01T00:00:00Z"}`), 0644)

	notes, _ := loadNotes()
	if notes[0].ID != 7 {
		t.Errorf("json key 'id' must map to ID field; got %d", notes[0].ID)
	}
}

func TestCompat_JSONKeys_TextField(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(`{"id":1,"text":"the text","created_at":"2024-01-01T00:00:00Z"}`), 0644)

	notes, _ := loadNotes()
	if notes[0].Text != "the text" {
		t.Errorf("json key 'text' must map to Text field; got %q", notes[0].Text)
	}
}

func TestCompat_JSONKeys_CreatedAtField(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(`{"id":1,"text":"x","created_at":"2024-06-01T00:00:00Z"}`), 0644)

	notes, _ := loadNotes()
	if notes[0].CreatedAt != "2024-06-01T00:00:00Z" {
		t.Errorf("json key 'created_at' must map to CreatedAt field; got %q", notes[0].CreatedAt)
	}
}

func TestCompat_JSONKeys_UpdatedAtField(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(`{"id":1,"text":"x","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-07-01T00:00:00Z"}`), 0644)

	notes, _ := loadNotes()
	if notes[0].UpdatedAt != "2024-07-01T00:00:00Z" {
		t.Errorf("json key 'updated_at' must map to UpdatedAt field; got %q", notes[0].UpdatedAt)
	}
}

func TestCompat_JSONKeys_TagsField(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(`{"id":1,"text":"x","created_at":"2024-01-01T00:00:00Z","tags":["foo"]}`), 0644)

	notes, _ := loadNotes()
	if len(notes[0].Tags) != 1 || notes[0].Tags[0] != "foo" {
		t.Errorf("json key 'tags' must map to Tags field; got %v", notes[0].Tags)
	}
}


func TestCompat_RoundTrip_PreservesAllFields(t *testing.T) {
	defer setupTempFile(t)()
	os.WriteFile(notesPathOverride, []byte(fixtureAllFields), 0644)

	notes, _ := loadNotes()
	if err := saveNotes(notes); err != nil {
		t.Fatalf("unexpected error on save: %v", err)
	}

	reloaded, err := loadNotes()
	if err != nil {
		t.Fatalf("unexpected error on reload: %v", err)
	}
	n := reloaded[0]
	if n.ID != 42 || n.Text != "full note" || n.CreatedAt != "2024-03-01T12:00:00Z" ||
		n.UpdatedAt != "2024-03-10T09:00:00Z" || len(n.Tags) != 2 {
		t.Errorf("round-trip data mismatch: %+v", n)
	}
}
