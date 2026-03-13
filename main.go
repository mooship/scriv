package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	var err error
	switch os.Args[1] {
	case "add":
		if len(os.Args) < 3 {
			fatalf("usage: jot add <text>")
		}
		err = cmdAdd(os.Args[2])
	case "list":
		err = cmdList()
	case "done":
		if len(os.Args) < 3 {
			fatalf("usage: jot done <id>")
		}
		err = cmdDone(os.Args[2])
	case "search":
		if len(os.Args) < 3 {
			fatalf("usage: jot search <query>")
		}
		err = cmdSearch(os.Args[2])
	case "clear":
		force := len(os.Args) > 2 && os.Args[2] == "--force"
		err = cmdClear(force)
	case "-h", "--help", "help":
		printUsage()
	case "-V", "--version", "version":
		fmt.Println("jot 0.1.0")
	default:
		fatalf("unknown command: %s\nRun 'jot --help' for usage.", os.Args[1])
	}

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s\n", err)
		os.Exit(1)
	}
}

func fatalf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "Error: "+format+"\n", args...)
	os.Exit(1)
}

func printUsage() {
	fmt.Print(`jot - Fast local note manager

Usage: jot <command> [arguments]

Commands:
  add <text>    Add a new note
  list          List all notes
  done <id>     Remove a note by id
  search <text> Search notes by text
  clear         Remove all notes

Options:
  -h, --help     Print help
  -V, --version  Print version
`)
}
