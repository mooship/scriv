package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"

	"golang.org/x/term"
)

func cmdAdd(text string) error {
	note, err := addNote(text)
	if err != nil {
		return err
	}
	fmt.Printf("Added [%d] %s\n", note.ID, note.Text)
	return nil
}

func cmdList(opts ListOptions) error {
	notes, err := listNotes(opts)
	if err != nil {
		return err
	}
	if len(notes) == 0 {
		fmt.Println("No notes.")
		return nil
	}
	for _, n := range notes {
		text := n.Text
		if !opts.Full && len(text) > 72 {
			text = text[:72] + "..."
		}
		line := fmt.Sprintf("[%d] (%s) %s", n.ID, noteAge(n.CreatedAt), text)
		if len(n.Tags) > 0 {
			line += " #" + strings.Join(n.Tags, " #")
		}
		fmt.Println(line)
	}
	fmt.Printf("%d notes.\n", len(notes))
	return nil
}

func cmdView(idStr string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := getNote(id)
	if err != nil {
		return err
	}
	fmt.Printf("[%d] %s\n", note.ID, note.Text)
	t, err := time.Parse("2006-01-02T15:04:05Z", note.CreatedAt)
	if err == nil {
		fmt.Printf("    Created: %s\n", t.Format("2006-01-02"))
	}
	if note.UpdatedAt != "" {
		u, err := time.Parse("2006-01-02T15:04:05Z", note.UpdatedAt)
		if err == nil {
			fmt.Printf("    Updated: %s\n", u.Format("2006-01-02"))
		}
	}
	if len(note.Tags) > 0 {
		fmt.Printf("    Tags: #%s\n", strings.Join(note.Tags, " #"))
	}
	return nil
}

func cmdDone(idStrs []string, force bool) error {
	var ids []uint64
	for _, s := range idStrs {
		id, err := strconv.ParseUint(s, 10, 64)
		if err != nil || id == 0 {
			return fmt.Errorf("id must be a positive integer: %s", s)
		}
		ids = append(ids, id)
	}
	removed, err := removeNotes(ids, force)
	if err != nil {
		return err
	}
	for _, n := range removed {
		fmt.Printf("Removed [%d] %s\n", n.ID, n.Text)
	}
	return nil
}

func cmdEdit(idStr string, text string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := editNote(id, text)
	if err != nil {
		return err
	}
	fmt.Printf("Updated [%d] %s\n", note.ID, note.Text)
	return nil
}

func noteAge(ts string) string {
	t, err := time.Parse("2006-01-02T15:04:05Z", ts)
	if err != nil {
		return "?"
	}
	d := time.Since(t)
	switch {
	case d < time.Hour:
		return "<1h"
	case d < 24*time.Hour:
		return fmt.Sprintf("%dh", int(d.Hours()))
	case d < 14*24*time.Hour:
		return fmt.Sprintf("%dd", int(d.Hours()/24))
	case d < 56*24*time.Hour:
		return fmt.Sprintf("%dw", int(d.Hours()/(7*24)))
	case d < 730*24*time.Hour:
		return fmt.Sprintf("%dmo", int(d.Hours()/(30*24)))
	default:
		return fmt.Sprintf("%dy", int(d.Hours()/(365*24)))
	}
}

func cmdTag(idStr string, tags []string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := tagNote(id, tags)
	if err != nil {
		return err
	}
	fmt.Printf("Tagged [%d] %s: #%s\n", note.ID, note.Text, strings.Join(note.Tags, " #"))
	return nil
}

func cmdUntag(idStr string, tag string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := untagNote(id, tag)
	if err != nil {
		return err
	}
	fmt.Printf("Removed tag #%s from [%d] %s\n", tag, note.ID, note.Text)
	return nil
}

func cmdTags() error {
	notes, err := loadNotes()
	if err != nil {
		return err
	}
	counts := collectTags(notes)
	if len(counts) == 0 {
		fmt.Println("No tags.")
		return nil
	}
	tags := make([]string, 0, len(counts))
	for t := range counts {
		tags = append(tags, t)
	}
	sort.Strings(tags)
	for _, t := range tags {
		fmt.Printf("%-20s %d\n", t, counts[t])
	}
	return nil
}

func cmdAppend(idStr string, text string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := appendNote(id, text)
	if err != nil {
		return err
	}
	fmt.Printf("Updated [%d] %s\n", note.ID, note.Text)
	return nil
}

func readStdinText(r io.Reader) (string, error) {
	scanner := bufio.NewScanner(r)
	var lines []string
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	text := strings.TrimSpace(strings.Join(lines, "\n"))
	if text == "" {
		return "", fmt.Errorf("no text provided via stdin")
	}
	return text, nil
}

func cmdClear(force bool) error {
	notes, err := loadNotes()
	if err != nil {
		return err
	}
	if len(notes) == 0 {
		fmt.Println("No notes.")
		return nil
	}
	if !force {
		fmt.Printf("Remove all %d notes? [y/N] ", len(notes))
		scanner := bufio.NewScanner(os.Stdin)
		scanner.Scan()
		if strings.ToLower(strings.TrimSpace(scanner.Text())) != "y" {
			return nil
		}
	}
	if err := clearNotes(); err != nil {
		return err
	}
	fmt.Println("Cleared.")
	return nil
}

func cmdSearch(query string) error {
	results, err := searchNotes(query)
	if err != nil {
		return err
	}
	if len(results) == 0 {
		fmt.Println("No matches.")
		return nil
	}
	useColor := stdoutIsTerminal()
	for _, n := range results {
		text := n.Text
		if useColor {
			text = highlightMatch(text, query)
		}
		fmt.Printf("[%d] %s\n", n.ID, text)
	}
	fmt.Printf("%d matches.\n", len(results))
	return nil
}

func cmdExport() error {
	notes, err := loadNotes()
	if err != nil {
		return err
	}
	for _, n := range notes {
		line, err := json.Marshal(n)
		if err != nil {
			return err
		}
		fmt.Println(string(line))
	}
	return nil
}

func cmdImport(r io.Reader) error {
	scanner := bufio.NewScanner(r)
	var incoming []Note
	lineNum := 0
	for scanner.Scan() {
		lineNum++
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var n Note
		if err := json.Unmarshal([]byte(line), &n); err != nil {
			return fmt.Errorf("line %d: invalid JSON: %s", lineNum, err)
		}
		incoming = append(incoming, n)
	}
	if err := scanner.Err(); err != nil {
		return err
	}
	if len(incoming) == 0 {
		fmt.Println("No notes to import.")
		return nil
	}
	if err := importNotes(incoming); err != nil {
		return err
	}
	fmt.Printf("Imported %d notes.\n", len(incoming))
	return nil
}

func stdoutIsTerminal() bool {
	fi, err := os.Stdout.Stat()
	if err != nil {
		return false
	}
	return (fi.Mode() & os.ModeCharDevice) != 0
}

// promptPassword prints msg to stderr and reads a password from the terminal without echo.
func promptPassword(msg string) (string, error) {
	fmt.Fprint(os.Stderr, msg)
	pw, err := term.ReadPassword(int(os.Stdin.Fd()))
	fmt.Fprintln(os.Stderr) // newline after hidden input
	if err != nil {
		return "", err
	}
	return string(pw), nil
}

func cmdLock() error {
	// If already encrypted, verify current password before changing.
	if notesFileIsEncrypted() {
		cur, err := promptPassword("Current password: ")
		if err != nil {
			return err
		}
		// Verify by attempting to load notes with this password.
		activePassword = cur
		if _, err := loadNotes(); err != nil {
			activePassword = ""
			return err
		}
	}

	// Prompt for new password with confirmation.
	pw, err := promptPassword("New password: ")
	if err != nil {
		return err
	}
	if pw == "" {
		return fmt.Errorf("password cannot be empty")
	}
	confirm, err := promptPassword("Confirm password: ")
	if err != nil {
		return err
	}
	if pw != confirm {
		return fmt.Errorf("passwords do not match")
	}

	// Load notes with current password (already set in activePassword), then re-save with new password.
	notes, err := loadNotes()
	if err != nil {
		return err
	}
	activePassword = pw
	if err := saveNotes(notes); err != nil {
		return err
	}
	fmt.Println("Notes are now password protected.")
	return nil
}

func cmdUnlock() error {
	if !notesFileIsEncrypted() {
		fmt.Println("Notes are not password protected.")
		return nil
	}
	pw, err := promptPassword("Password: ")
	if err != nil {
		return err
	}
	activePassword = pw
	notes, err := loadNotes()
	if err != nil {
		activePassword = ""
		return err
	}
	// Save as plaintext by clearing activePassword before save.
	activePassword = ""
	if err := saveNotes(notes); err != nil {
		return err
	}
	fmt.Println("Password protection removed.")
	return nil
}

func highlightMatch(text, query string) string {
	lower := strings.ToLower(text)
	lowerQ := strings.ToLower(query)
	var result strings.Builder
	i := 0
	for {
		idx := strings.Index(lower[i:], lowerQ)
		if idx == -1 {
			result.WriteString(text[i:])
			break
		}
		result.WriteString(text[i : i+idx])
		result.WriteString("\033[1;33m")
		result.WriteString(text[i+idx : i+idx+len(lowerQ)])
		result.WriteString("\033[0m")
		i += idx + len(lowerQ)
	}
	return result.String()
}
