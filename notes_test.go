package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
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

	json := `{"id":1,"text":"hello","created_at":"2025-01-01T00:00:00Z"}`
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

func TestEditNote_UpdatesText(t *testing.T) {
	defer setupTempFile(t)()

	addNote("original")

	updated, err := editNote(1, "revised")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if updated.Text != "revised" {
		t.Errorf("expected revised, got %s", updated.Text)
	}
}

func TestEditNote_PersistsToDisk(t *testing.T) {
	defer setupTempFile(t)()

	addNote("original")
	editNote(1, "revised")

	notes, _ := loadNotes()
	if notes[0].Text != "revised" {
		t.Errorf("expected revised on disk, got %s", notes[0].Text)
	}
}

func TestEditNote_PreservesID(t *testing.T) {
	defer setupTempFile(t)()

	addNote("original")
	updated, _ := editNote(1, "revised")
	if updated.ID != 1 {
		t.Errorf("expected ID 1, got %d", updated.ID)
	}
}

func TestEditNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")

	_, err := editNote(99, "new text")
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
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

func TestGetNote_ReturnsCorrectNote(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")

	note, err := getNote(2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.ID != 2 || note.Text != "beta" {
		t.Errorf("unexpected note: %+v", note)
	}
}

func TestGetNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")

	_, err := getNote(99)
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestCmdView_InvalidIDString(t *testing.T) {
	defer setupTempFile(t)()

	err := cmdView("abc")
	if err == nil {
		t.Fatal("expected error for non-integer ID")
	}
}

func TestCmdView_ValidIDShowsNote(t *testing.T) {
	defer setupTempFile(t)()

	addNote("check details")

	if err := cmdView("1"); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestCmdDone_InvalidIDString(t *testing.T) {
	defer setupTempFile(t)()

	err := cmdDone([]string{"abc"})
	if err == nil {
		t.Fatal("expected error for non-integer ID")
	}
}

func TestCmdDone_ZeroIDIsInvalid(t *testing.T) {
	defer setupTempFile(t)()

	err := cmdDone([]string{"0"})
	if err == nil {
		t.Fatal("expected error for ID 0")
	}
}

func TestCmdDone_ValidIDRemovesNote(t *testing.T) {
	defer setupTempFile(t)()

	addNote("to remove")

	if err := cmdDone([]string{"1"}); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	notes, _ := loadNotes()
	if len(notes) != 0 {
		t.Error("expected note to be removed")
	}
}

func TestAddNote_UpdatedAtIsEmpty(t *testing.T) {
	defer setupTempFile(t)()

	note, _ := addNote("test")
	if note.UpdatedAt != "" {
		t.Errorf("expected UpdatedAt empty after add, got %q", note.UpdatedAt)
	}
}

func TestEditNote_SetsUpdatedAt(t *testing.T) {
	defer setupTempFile(t)()

	addNote("original")
	updated, err := editNote(1, "revised")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if updated.UpdatedAt == "" {
		t.Error("expected UpdatedAt to be set after edit")
	}
	if _, err := time.Parse("2006-01-02T15:04:05Z", updated.UpdatedAt); err != nil {
		t.Errorf("UpdatedAt not a valid timestamp: %q", updated.UpdatedAt)
	}
}

func TestTagNote_AddsSingleTag(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	note, err := tagNote(1, []string{"groceries"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(note.Tags) != 1 || note.Tags[0] != "groceries" {
		t.Errorf("unexpected tags: %v", note.Tags)
	}
}

func TestTagNote_AddsMultipleTags(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	note, err := tagNote(1, []string{"groceries", "shopping"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(note.Tags) != 2 {
		t.Errorf("expected 2 tags, got %v", note.Tags)
	}
}

func TestTagNote_DeduplicatesTags(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})
	note, err := tagNote(1, []string{"groceries"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(note.Tags) != 1 {
		t.Errorf("expected 1 tag after dedup, got %v", note.Tags)
	}
}

func TestTagNote_PersistsToDisk(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})

	notes, _ := loadNotes()
	if len(notes[0].Tags) != 1 || notes[0].Tags[0] != "groceries" {
		t.Errorf("tags not persisted: %v", notes[0].Tags)
	}
}

func TestTagNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")
	_, err := tagNote(99, []string{"tag"})
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestUntagNote_RemovesTag(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})
	note, err := untagNote(1, "groceries")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(note.Tags) != 0 {
		t.Errorf("expected no tags, got %v", note.Tags)
	}
}

func TestUntagNote_LeavesOtherTagsIntact(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries", "shopping"})
	note, err := untagNote(1, "shopping")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(note.Tags) != 1 || note.Tags[0] != "groceries" {
		t.Errorf("unexpected tags after untag: %v", note.Tags)
	}
}

func TestUntagNote_NonexistentTagIsNoop(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})
	note, err := untagNote(1, "nonexistent")
	if err != nil {
		t.Fatalf("expected no error for missing tag, got: %v", err)
	}
	if len(note.Tags) != 1 {
		t.Errorf("expected tags unchanged, got %v", note.Tags)
	}
}

func TestUntagNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")
	_, err := untagNote(99, "tag")
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestListNotes_SortByID(t *testing.T) {
	defer setupTempFile(t)()

	addNote("first")
	addNote("second")
	addNote("third")

	notes, err := listNotes(ListOptions{Sort: "id"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if notes[0].ID != 1 || notes[1].ID != 2 || notes[2].ID != 3 {
		t.Errorf("unexpected order: %v", []uint64{notes[0].ID, notes[1].ID, notes[2].ID})
	}
}

func TestListNotes_SortByDate(t *testing.T) {
	defer setupTempFile(t)()

	addNote("first")
	addNote("second")

	notes, err := listNotes(ListOptions{Sort: "date"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 2 {
		t.Errorf("expected 2 notes, got %d", len(notes))
	}
}

func TestListNotes_SortByUpdated(t *testing.T) {
	defer setupTempFile(t)()

	addNote("first")
	addNote("second")
	editNote(1, "first edited")

	notes, err := listNotes(ListOptions{Sort: "updated"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if notes[0].ID != 1 {
		t.Errorf("expected edited note first, got ID %d", notes[0].ID)
	}
}

func TestListNotes_SortByUpdatedFallsBackToCreatedAt(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")

	notes, err := listNotes(ListOptions{Sort: "updated"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 2 {
		t.Errorf("expected 2 notes, got %d", len(notes))
	}
}

func TestListNotes_UnknownSortReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("test")
	_, err := listNotes(ListOptions{Sort: "bogus"})
	if err == nil {
		t.Fatal("expected error for unknown sort, got nil")
	}
}

func TestListNotes_FilterByTag_MatchesTagged(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	addNote("fix bug")
	tagNote(1, []string{"groceries"})

	notes, err := listNotes(ListOptions{Tag: "groceries"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 1 || notes[0].ID != 1 {
		t.Errorf("expected only note 1, got %+v", notes)
	}
}

func TestListNotes_FilterByTag_NoMatches(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})

	notes, err := listNotes(ListOptions{Tag: "work"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 0 {
		t.Errorf("expected no notes, got %d", len(notes))
	}
}

func TestListNotes_FilterByTag_EmptyTagMeansAll(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")

	notes, err := listNotes(ListOptions{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 2 {
		t.Errorf("expected 2 notes, got %d", len(notes))
	}
}

func TestSearchNotes_MatchesTag(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"groceries"})

	results, err := searchNotes("groc")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 || results[0].ID != 1 {
		t.Errorf("expected note 1 to match via tag, got %+v", results)
	}
}

func TestSearchNotes_TagMatchCaseInsensitive(t *testing.T) {
	defer setupTempFile(t)()

	addNote("buy milk")
	tagNote(1, []string{"Groceries"})

	results, err := searchNotes("groc")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 {
		t.Errorf("expected case-insensitive tag match, got %+v", results)
	}
}

func TestReadStdinText_ReturnsJoinedLines(t *testing.T) {
	r := strings.NewReader("line one\nline two\n")
	text, err := readStdinText(r)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if text != "line one\nline two" {
		t.Errorf("unexpected text: %q", text)
	}
}

func TestReadStdinText_TrimsWhitespace(t *testing.T) {
	r := strings.NewReader("  hello  \n")
	text, err := readStdinText(r)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if text != "hello" {
		t.Errorf("expected trimmed text, got %q", text)
	}
}

func TestReadStdinText_EmptyInputReturnsError(t *testing.T) {
	r := strings.NewReader("   \n")
	_, err := readStdinText(r)
	if err == nil {
		t.Fatal("expected error for empty input, got nil")
	}
}

func TestCollectTags_EmptyNotes(t *testing.T) {
	counts := collectTags([]Note{})
	if len(counts) != 0 {
		t.Errorf("expected empty map, got %v", counts)
	}
}

func TestCollectTags_CountsCorrectly(t *testing.T) {
	notes := []Note{
		{ID: 1, Text: "a", Tags: []string{"bug", "work"}},
		{ID: 2, Text: "b", Tags: []string{"bug"}},
		{ID: 3, Text: "c", Tags: []string{"feature"}},
	}
	counts := collectTags(notes)
	if counts["bug"] != 2 {
		t.Errorf("expected bug count 2, got %d", counts["bug"])
	}
	if counts["work"] != 1 {
		t.Errorf("expected work count 1, got %d", counts["work"])
	}
	if counts["feature"] != 1 {
		t.Errorf("expected feature count 1, got %d", counts["feature"])
	}
}

func TestCollectTags_NoTagsReturnsEmptyMap(t *testing.T) {
	notes := []Note{
		{ID: 1, Text: "no tags here"},
	}
	counts := collectTags(notes)
	if len(counts) != 0 {
		t.Errorf("expected empty map for untagged notes, got %v", counts)
	}
}

func TestAppendNote_AppendsText(t *testing.T) {
	defer setupTempFile(t)()

	addNote("hello")
	note, err := appendNote(1, "world")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.Text != "hello world" {
		t.Errorf("expected 'hello world', got %q", note.Text)
	}
}

func TestAppendNote_SetsUpdatedAt(t *testing.T) {
	defer setupTempFile(t)()

	addNote("hello")
	note, err := appendNote(1, "world")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if note.UpdatedAt == "" {
		t.Error("expected UpdatedAt to be set after append")
	}
}

func TestAppendNote_PersistsToDisk(t *testing.T) {
	defer setupTempFile(t)()

	addNote("hello")
	appendNote(1, "world")

	notes, _ := loadNotes()
	if notes[0].Text != "hello world" {
		t.Errorf("expected persisted text 'hello world', got %q", notes[0].Text)
	}
}

func TestAppendNote_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")
	_, err := appendNote(99, "more")
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestRemoveNotes_RemovesMultiple(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")
	addNote("gamma")

	removed, err := removeNotes([]uint64{1, 3})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(removed) != 2 {
		t.Fatalf("expected 2 removed, got %d", len(removed))
	}

	notes, _ := loadNotes()
	if len(notes) != 1 || notes[0].Text != "beta" {
		t.Errorf("expected only beta to remain, got %+v", notes)
	}
}

func TestRemoveNotes_NotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("only note")

	_, err := removeNotes([]uint64{99})
	if err == nil {
		t.Fatal("expected error for missing ID, got nil")
	}
}

func TestRemoveNotes_PartialNotFoundReturnsError(t *testing.T) {
	defer setupTempFile(t)()

	addNote("alpha")
	addNote("beta")

	_, err := removeNotes([]uint64{1, 99})
	if err == nil {
		t.Fatal("expected error when one ID is missing, got nil")
	}
}

func TestListNotes_LimitCapsResults(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")
	addNote("three")

	notes, err := listNotes(ListOptions{Limit: 2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 2 {
		t.Errorf("expected 2 notes with limit=2, got %d", len(notes))
	}
}

func TestListNotes_LimitZeroMeansAll(t *testing.T) {
	defer setupTempFile(t)()

	addNote("one")
	addNote("two")
	addNote("three")

	notes, err := listNotes(ListOptions{Limit: 0})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(notes) != 3 {
		t.Errorf("expected 3 notes with no limit, got %d", len(notes))
	}
}

func TestNoteAge_ReturnsNonEmpty(t *testing.T) {
	ts := time.Now().UTC().Add(-48 * time.Hour).Format("2006-01-02T15:04:05Z")
	age := noteAge(ts)
	if age == "" || age == "?" {
		t.Errorf("expected non-empty age, got %q", age)
	}
}

func TestNoteAge_InvalidTimestamp(t *testing.T) {
	age := noteAge("not-a-timestamp")
	if age != "?" {
		t.Errorf("expected '?' for invalid timestamp, got %q", age)
	}
}

var _ = time.Now
