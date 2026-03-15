package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestEncryptDecryptRoundtrip(t *testing.T) {
	plaintext := []byte(`{"id":1,"text":"hello","created_at":"2024-01-01T00:00:00Z"}` + "\n")
	encrypted, err := encryptNotes(plaintext, "secret")
	if err != nil {
		t.Fatalf("encryptNotes: %v", err)
	}
	if string(encrypted[:len(encryptedMagic)]) != encryptedMagic {
		t.Error("encrypted output missing magic header")
	}
	decrypted, err := decryptNotes(encrypted, "secret")
	if err != nil {
		t.Fatalf("decryptNotes: %v", err)
	}
	if string(decrypted) != string(plaintext) {
		t.Errorf("roundtrip mismatch: got %q, want %q", decrypted, plaintext)
	}
}

func TestDecryptWrongPassword(t *testing.T) {
	plaintext := []byte(`{"id":1,"text":"hello","created_at":"2024-01-01T00:00:00Z"}` + "\n")
	encrypted, err := encryptNotes(plaintext, "correct")
	if err != nil {
		t.Fatalf("encryptNotes: %v", err)
	}
	_, err = decryptNotes(encrypted, "wrong")
	if err == nil {
		t.Fatal("expected error with wrong password, got nil")
	}
	if err.Error() != "incorrect password" {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestDecryptTruncated(t *testing.T) {
	_, err := decryptNotes([]byte("JOT\x01short"), "any")
	if err == nil {
		t.Fatal("expected error for truncated data, got nil")
	}
}

func TestDecryptNotMagic(t *testing.T) {
	_, err := decryptNotes([]byte(`{"id":1}`), "any")
	if err == nil {
		t.Fatal("expected error for non-encrypted data, got nil")
	}
}

func TestIsEncryptedData_plain(t *testing.T) {
	plain := []byte(`{"id":1,"text":"hi","created_at":"2024-01-01T00:00:00Z"}` + "\n")
	if isEncryptedData(plain) {
		t.Error("plain NDJSON should not be detected as encrypted")
	}
}

func TestIsEncryptedData_encrypted(t *testing.T) {
	encrypted, err := encryptNotes([]byte("test"), "pw")
	if err != nil {
		t.Fatalf("encryptNotes: %v", err)
	}
	if !isEncryptedData(encrypted) {
		t.Error("encrypted data should be detected as encrypted")
	}
}

func TestNotesFileIsEncrypted_plain(t *testing.T) {
	defer setupTempFile(t)()
	// No file yet — should return false.
	if notesFileIsEncrypted() {
		t.Error("missing file should not be detected as encrypted")
	}
	// Write plain NDJSON.
	if err := os.WriteFile(notesPath(), []byte(`{"id":1,"text":"hi","created_at":"2024-01-01T00:00:00Z"}`+"\n"), 0600); err != nil {
		t.Fatalf("WriteFile: %v", err)
	}
	if notesFileIsEncrypted() {
		t.Error("plain NDJSON file should not be detected as encrypted")
	}
}

func TestNotesFileIsEncrypted_encrypted(t *testing.T) {
	defer setupTempFile(t)()
	encrypted, err := encryptNotes([]byte("test"), "pw")
	if err != nil {
		t.Fatalf("encryptNotes: %v", err)
	}
	dir := filepath.Dir(notesPath())
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	if err := os.WriteFile(notesPath(), encrypted, 0600); err != nil {
		t.Fatalf("WriteFile: %v", err)
	}
	if !notesFileIsEncrypted() {
		t.Error("encrypted file should be detected as encrypted")
	}
}

func TestSaveLoadEncryptedNotes(t *testing.T) {
	defer setupTempFile(t)()
	activePassword = "testpassword"
	defer func() { activePassword = "" }()

	want := []Note{
		{ID: 1, Text: "secret note", CreatedAt: "2024-01-01T00:00:00Z"},
		{ID: 2, Text: "another secret", CreatedAt: "2024-02-01T00:00:00Z", Tags: []string{"private"}},
	}
	if err := saveNotes(want); err != nil {
		t.Fatalf("saveNotes: %v", err)
	}

	// File should be encrypted on disk.
	raw, err := os.ReadFile(notesPath())
	if err != nil {
		t.Fatalf("ReadFile: %v", err)
	}
	if !isEncryptedData(raw) {
		t.Error("saved file should be encrypted")
	}

	got, err := loadNotes()
	if err != nil {
		t.Fatalf("loadNotes: %v", err)
	}
	if len(got) != len(want) {
		t.Fatalf("got %d notes, want %d", len(got), len(want))
	}
	for i := range want {
		if got[i].ID != want[i].ID || got[i].Text != want[i].Text {
			t.Errorf("note[%d]: got %+v, want %+v", i, got[i], want[i])
		}
	}
}

func TestSaveLoadNoPassword(t *testing.T) {
	defer setupTempFile(t)()
	// Ensure activePassword is empty (default behaviour).
	activePassword = ""

	want := []Note{
		{ID: 1, Text: "plain note", CreatedAt: "2024-01-01T00:00:00Z"},
	}
	if err := saveNotes(want); err != nil {
		t.Fatalf("saveNotes: %v", err)
	}
	raw, err := os.ReadFile(notesPath())
	if err != nil {
		t.Fatalf("ReadFile: %v", err)
	}
	if isEncryptedData(raw) {
		t.Error("file should be plain NDJSON when no password is set")
	}
	got, err := loadNotes()
	if err != nil {
		t.Fatalf("loadNotes: %v", err)
	}
	if len(got) != 1 || got[0].Text != "plain note" {
		t.Errorf("unexpected notes: %+v", got)
	}
}

func TestEncryptEmptyNotes(t *testing.T) {
	defer setupTempFile(t)()
	activePassword = "pw"
	defer func() { activePassword = "" }()

	if err := saveNotes([]Note{}); err != nil {
		t.Fatalf("saveNotes empty: %v", err)
	}
	got, err := loadNotes()
	if err != nil {
		t.Fatalf("loadNotes: %v", err)
	}
	if len(got) != 0 {
		t.Errorf("expected empty notes, got %d", len(got))
	}
}

func TestLoadEncryptedWrongPassword(t *testing.T) {
	defer setupTempFile(t)()
	activePassword = "correct"
	defer func() { activePassword = "" }()

	if err := saveNotes([]Note{{ID: 1, Text: "hi", CreatedAt: "2024-01-01T00:00:00Z"}}); err != nil {
		t.Fatalf("saveNotes: %v", err)
	}

	activePassword = "wrong"
	_, err := loadNotes()
	if err == nil {
		t.Fatal("expected error loading with wrong password")
	}
	if err.Error() != "incorrect password" {
		t.Errorf("unexpected error: %v", err)
	}
}
