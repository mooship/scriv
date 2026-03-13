package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

type Note struct {
	ID        uint64 `json:"id"`
	Text      string `json:"text"`
	CreatedAt string `json:"created_at"`
}

var notesPathOverride string

func notesPath() string {
	if notesPathOverride != "" {
		return notesPathOverride
	}
	var dataDir string
	switch runtime.GOOS {
	case "windows":
		dataDir = os.Getenv("APPDATA")
	case "darwin":
		home, _ := os.UserHomeDir()
		dataDir = filepath.Join(home, "Library", "Application Support")
	default:
		dataDir = os.Getenv("XDG_DATA_HOME")
		if dataDir == "" {
			home, _ := os.UserHomeDir()
			dataDir = filepath.Join(home, ".local", "share")
		}
	}
	if dataDir == "" {
		dataDir = "."
	}
	return filepath.Join(dataDir, "jot", "notes.json")
}

func loadNotes() ([]Note, error) {
	path := notesPath()
	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		return []Note{}, nil
	}
	if err != nil {
		return nil, fmt.Errorf("cannot write to %s: %w", path, err)
	}
	var notes []Note
	if err := json.Unmarshal(data, &notes); err != nil {
		return nil, fmt.Errorf("notes file is corrupted. Run 'jot clear --force' to reset.")
	}
	return notes, nil
}

func saveNotes(notes []Note) error {
	path := notesPath()
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return fmt.Errorf("cannot write to %s: %w", filepath.Dir(path), err)
	}
	data, err := json.MarshalIndent(notes, "", "  ")
	if err != nil {
		return err
	}
	if err := os.WriteFile(path, data, 0644); err != nil {
		return fmt.Errorf("cannot write to %s: %w", path, err)
	}
	return nil
}

func addNote(text string) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	var maxID uint64
	for _, n := range notes {
		if n.ID > maxID {
			maxID = n.ID
		}
	}
	note := Note{
		ID:        maxID + 1,
		Text:      text,
		CreatedAt: time.Now().UTC().Format("2006-01-02T15:04:05Z"),
	}
	notes = append(notes, note)
	return note, saveNotes(notes)
}

func removeNote(id uint64) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	for i, n := range notes {
		if n.ID == id {
			notes = append(notes[:i], notes[i+1:]...)
			return n, saveNotes(notes)
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

func searchNotes(query string) ([]Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return nil, err
	}
	q := strings.ToLower(query)
	var results []Note
	for _, n := range notes {
		if strings.Contains(strings.ToLower(n.Text), q) {
			results = append(results, n)
		}
	}
	return results, nil
}

func editNote(id uint64, text string) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	for i, n := range notes {
		if n.ID == id {
			notes[i].Text = text
			return notes[i], saveNotes(notes)
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

func clearNotes() error {
	return saveNotes([]Note{})
}
