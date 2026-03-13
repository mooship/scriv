package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func cmdAdd(text string) error {
	note, err := addNote(text)
	if err != nil {
		return err
	}
	fmt.Printf("Added [%d] %s\n", note.ID, note.Text)
	return nil
}

func cmdList() error {
	notes, err := loadNotes()
	if err != nil {
		return err
	}
	if len(notes) == 0 {
		fmt.Println("No notes.")
		return nil
	}
	for _, n := range notes {
		fmt.Printf("[%d] %s\n", n.ID, n.Text)
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
