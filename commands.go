package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"
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
		if len(text) > 72 {
			text = text[:72] + "..."
		}
		line := fmt.Sprintf("[%d] %s", n.ID, text)
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

func cmdDone(idStr string) error {
	id, err := strconv.ParseUint(idStr, 10, 64)
	if err != nil || id == 0 {
		return fmt.Errorf("id must be a positive integer")
	}
	note, err := removeNote(id)
	if err != nil {
		return err
	}
	fmt.Printf("Removed [%d] %s\n", note.ID, note.Text)
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

func cmdSearch(query string) error {
	results, err := searchNotes(query)
	if err != nil {
		return err
	}
	if len(results) == 0 {
		fmt.Println("No matches.")
		return nil
	}
	for _, n := range results {
		fmt.Printf("[%d] %s\n", n.ID, n.Text)
	}
	return nil
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
