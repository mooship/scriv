package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"time"
)

type Note struct {
	ID        uint64   `json:"id"`
	Text      string   `json:"text"`
	CreatedAt string   `json:"created_at"`
	UpdatedAt string   `json:"updated_at,omitempty"`
	Tags      []string `json:"tags,omitempty"`
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
		return nil, fmt.Errorf("cannot read from %s: %w", path, err)
	}
	var notes []Note
	scanner := bufio.NewScanner(bytes.NewReader(data))
	for scanner.Scan() {
		line := bytes.TrimSpace(scanner.Bytes())
		if len(line) == 0 {
			continue
		}
		var n Note
		if err := json.Unmarshal(line, &n); err != nil {
			return nil, fmt.Errorf("notes file is corrupted. Run 'jot clear --force' to reset.")
		}
		notes = append(notes, n)
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("cannot read from %s: %w", path, err)
	}
	if notes == nil {
		return []Note{}, nil
	}
	return notes, nil
}

func saveNotes(notes []Note) error {
	path := notesPath()
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("cannot write to %s: %w", dir, err)
	}
	var buf bytes.Buffer
	for _, n := range notes {
		line, err := json.Marshal(n)
		if err != nil {
			return err
		}
		buf.Write(line)
		buf.WriteByte('\n')
	}
	tmp, err := os.CreateTemp(dir, "notes-*.json")
	if err != nil {
		return fmt.Errorf("cannot write to %s: %w", dir, err)
	}
	tmpName := tmp.Name()
	if _, err := tmp.Write(buf.Bytes()); err != nil {
		tmp.Close()
		os.Remove(tmpName)
		return fmt.Errorf("cannot write to %s: %w", tmpName, err)
	}
	if err := tmp.Close(); err != nil {
		os.Remove(tmpName)
		return fmt.Errorf("cannot write to %s: %w", tmpName, err)
	}
	if err := os.Rename(tmpName, path); err != nil {
		os.Remove(tmpName)
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
		matched := strings.Contains(strings.ToLower(n.Text), q)
		if !matched {
			for _, tag := range n.Tags {
				if strings.Contains(strings.ToLower(tag), q) {
					matched = true
					break
				}
			}
		}
		if matched {
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
			notes[i].UpdatedAt = time.Now().UTC().Format("2006-01-02T15:04:05Z")
			return notes[i], saveNotes(notes)
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

func getNote(id uint64) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	for _, n := range notes {
		if n.ID == id {
			return n, nil
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

func clearNotes() error {
	return saveNotes([]Note{})
}

func tagNote(id uint64, tags []string) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	for i, n := range notes {
		if n.ID == id {
			for _, newTag := range tags {
				found := false
				for _, existing := range notes[i].Tags {
					if existing == newTag {
						found = true
						break
					}
				}
				if !found {
					notes[i].Tags = append(notes[i].Tags, newTag)
				}
			}
			return notes[i], saveNotes(notes)
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

func untagNote(id uint64, tag string) (Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return Note{}, err
	}
	for i, n := range notes {
		if n.ID == id {
			filtered := notes[i].Tags[:0]
			for _, t := range notes[i].Tags {
				if t != tag {
					filtered = append(filtered, t)
				}
			}
			notes[i].Tags = filtered
			return notes[i], saveNotes(notes)
		}
	}
	return Note{}, fmt.Errorf("no note with id %d", id)
}

type ListOptions struct {
	Tag  string
	Sort string
}

func listNotes(opts ListOptions) ([]Note, error) {
	notes, err := loadNotes()
	if err != nil {
		return nil, err
	}
	if opts.Tag != "" {
		var filtered []Note
		for _, n := range notes {
			for _, t := range n.Tags {
				if t == opts.Tag {
					filtered = append(filtered, n)
					break
				}
			}
		}
		notes = filtered
	}
	switch opts.Sort {
	case "", "id":
		sort.Slice(notes, func(i, j int) bool {
			return notes[i].ID < notes[j].ID
		})
	case "date":
		sort.Slice(notes, func(i, j int) bool {
			return notes[i].CreatedAt > notes[j].CreatedAt
		})
	case "updated":
		key := func(n Note) string {
			if n.UpdatedAt != "" {
				return n.UpdatedAt
			}
			return n.CreatedAt
		}
		sort.Slice(notes, func(i, j int) bool {
			return key(notes[i]) > key(notes[j])
		})
	default:
		return nil, fmt.Errorf("unknown sort %q: use id, date, or updated", opts.Sort)
	}
	return notes, nil
}
