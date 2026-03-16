package main

import (
	"fmt"
	"os"
	"runtime/debug"
	"strconv"
	"strings"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	noPromptCmds := map[string]bool{
		"lock": true, "unlock": true,
		"-h": true, "--help": true, "help": true,
		"-V": true, "--version": true, "version": true,
	}
	if notesFileIsEncrypted() && !noPromptCmds[os.Args[1]] {
		pw, err := promptPassword("Password: ")
		if err != nil {
			fatalf("cannot read password: %s", err)
		}
		activePassword = pw
	}

	var err error
	switch os.Args[1] {
	case "add":
		var text string
		if stdinIsPiped() {
			text, err = readStdinText(os.Stdin)
			if err != nil {
				fatalf("%s", err)
			}
		} else {
			if len(os.Args) < 3 {
				fatalf("usage: jot add <text>")
			}
			text = strings.Join(os.Args[2:], " ")
		}
		err = cmdAdd(text)
	case "list":
		var opts ListOptions
		for _, arg := range os.Args[2:] {
			if strings.HasPrefix(arg, "--tag=") {
				opts.Tag = strings.TrimPrefix(arg, "--tag=")
			} else if strings.HasPrefix(arg, "--sort=") {
				opts.Sort = strings.TrimPrefix(arg, "--sort=")
			} else if strings.HasPrefix(arg, "--limit=") {
				n, e := strconv.Atoi(strings.TrimPrefix(arg, "--limit="))
				if e != nil || n < 1 {
					fatalf("--limit must be a positive integer")
				}
				opts.Limit = n
			} else if arg == "--full" {
				opts.Full = true
			} else {
				fatalf("unknown flag: %s", arg)
			}
		}
		err = cmdList(opts)
	case "edit":
		if len(os.Args) < 3 {
			fatalf("usage: jot edit <id> <text>")
		}
		var editText string
		if stdinIsPiped() {
			editText, err = readStdinText(os.Stdin)
			if err != nil {
				fatalf("%s", err)
			}
		} else {
			if len(os.Args) < 4 {
				fatalf("usage: jot edit <id> <text>")
			}
			editText = strings.Join(os.Args[3:], " ")
		}
		err = cmdEdit(os.Args[2], editText)
	case "done":
		force := false
		var doneArgs []string
		for _, arg := range os.Args[2:] {
			if arg == "--force" {
				force = true
			} else {
				doneArgs = append(doneArgs, arg)
			}
		}
		if len(doneArgs) == 0 {
			fatalf("usage: jot done [--force] <id> [id2...]")
		}
		err = cmdDone(doneArgs, force)
	case "search":
		if len(os.Args) < 3 {
			fatalf("usage: jot search <query>")
		}
		err = cmdSearch(strings.Join(os.Args[2:], " "))
	case "view":
		if len(os.Args) < 3 {
			fatalf("usage: jot view <id>")
		}
		err = cmdView(os.Args[2])
	case "tag":
		if len(os.Args) < 4 {
			fatalf("usage: jot tag <id> <tag1> [tag2...]")
		}
		err = cmdTag(os.Args[2], os.Args[3:])
	case "untag":
		if len(os.Args) < 4 {
			fatalf("usage: jot untag <id> <tag>")
		}
		err = cmdUntag(os.Args[2], os.Args[3])
	case "tags":
		err = cmdTags()
	case "append":
		if len(os.Args) < 4 {
			fatalf("usage: jot append <id> <text>")
		}
		err = cmdAppend(os.Args[2], strings.Join(os.Args[3:], " "))
	case "export":
		err = cmdExport()
	case "import":
		err = cmdImport(os.Stdin)
	case "clear":
		force := len(os.Args) > 2 && os.Args[2] == "--force"
		err = cmdClear(force)
	case "lock":
		err = cmdLock()
	case "unlock":
		err = cmdUnlock()
	case "-h", "--help", "help":
		printUsage()
	case "-V", "--version", "version":
		fmt.Println("jot " + appVersion())
	default:
		fatalf("unknown command: %s\nRun 'jot --help' for usage.", os.Args[1])
	}

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s\n", err)
		os.Exit(1)
	}
}

func appVersion() string {
	if info, ok := debug.ReadBuildInfo(); ok {
		v := info.Main.Version
		if v != "" && v != "(devel)" {
			return v
		}
	}
	return "dev"
}

func stdinIsPiped() bool {
	fi, err := os.Stdin.Stat()
	if err != nil {
		return false
	}
	return (fi.Mode() & os.ModeCharDevice) == 0
}

func fatalf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "Error: "+format+"\n", args...)
	os.Exit(1)
}

const usageTemplate = `jot - Fast local note manager

Version: %s

Usage: jot <command> [arguments]

Commands:
	add <text>              Add a new note (or pipe text via stdin)
	list [--tag=<tag>] [--sort=id|date|updated] [--limit=N] [--full]
													List notes, optionally filtered, sorted, and limited
	edit <id> <text>        Edit a note by id (or pipe new text via stdin)
	append <id> <text>      Append text to an existing note
	done [--force] <id> [id2...]
													Remove one or more notes by id (--force skips missing)
	search <text>           Search notes by text or tag
	view <id>               View full details of a note
	tag <id> <tag1> [...]   Add tags to a note
	untag <id> <tag>        Remove a tag from a note
	tags                    List all tags with note counts
	export                  Print all notes as NDJSON to stdout
	import                  Read NDJSON from stdin and append notes
	clear                   Remove all notes
	lock                    Set or change the notes password
	unlock                  Remove password protection

Options:
	-h, --help     Print help
	-V, --version  Print version
`

func printUsage() {
	fmt.Printf(usageTemplate, appVersion())
}
