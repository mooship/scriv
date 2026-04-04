//! CLI entrypoint and command wiring.

use chrono::{DateTime, Utc};
use scriv::{
    ListOptions, Note, add_note, append_note, clear_notes, collect_tags, edit_note, get_note,
    has_active_password, highlight_match, import_notes, list_notes, load_notes, note_age,
    notes_file_is_encrypted, read_stdin_text, remove_notes, search_notes, set_active_password,
    tag_note, untag_note,
};
use std::collections::BTreeMap;
use std::env;
use std::io::{self, BufRead, Read, Write};

const USAGE_TEMPLATE: &str = "scriv - Fast local note manager

Version: {version}

Usage: scriv <command> [arguments]

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
";

/// Return crate version embedded at compile time.
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Print consistent CLI help text.
fn print_usage() {
    println!("{}", USAGE_TEMPLATE.replace("{version}", app_version()));
}

/// Print an error and exit with status 1.
fn fatal(msg: &str) -> ! {
    eprintln!("Error: {}", msg);
    std::process::exit(1);
}

/// Parse a required positive note id.
fn parse_id(s: &str) -> Result<u64, String> {
    let id = s
        .parse::<u64>()
        .map_err(|_| "id must be a positive integer".to_string())?;
    if id == 0 {
        return Err("id must be a positive integer".to_string());
    }
    Ok(id)
}

/// True when stdin is piped rather than interactive.
fn stdin_is_piped() -> bool {
    !std::io::IsTerminal::is_terminal(&io::stdin())
}

/// Read text from piped stdin or from positional args starting at `start`.
fn text_from_stdin_or_args(args: &[String], start: usize) -> Result<String, String> {
    if stdin_is_piped() {
        return read_stdin_text(io::stdin());
    }
    if args.len() <= start {
        return Err("no text provided".to_string());
    }
    Ok(args[start..].join(" "))
}

/// True when stdout is attached to a terminal.
fn stdout_is_terminal() -> bool {
    std::io::IsTerminal::is_terminal(&io::stdout())
}

/// Prompt for a password without echoing input.
fn prompt_password(msg: &str) -> Result<String, String> {
    eprint!("{}", msg);
    rpassword::read_password().map_err(|e| e.to_string())
}

fn cmd_add(text: String) -> Result<(), String> {
    let note = add_note(&text)?;
    println!("Added [{}] {}", note.id, note.text);
    Ok(())
}

fn cmd_list(opts: ListOptions) -> Result<(), String> {
    let notes = list_notes(&opts)?;
    if notes.is_empty() {
        println!("No notes.");
        return Ok(());
    }

    for note in &notes {
        let mut text = note.text.clone();
        if !opts.full && text.chars().count() > 72 {
            text = text.chars().take(72).collect::<String>() + "...";
        }

        let mut line = format!("[{}] ({}) {}", note.id, note_age(&note.created_at), text);
        if !note.tags.is_empty() {
            line.push_str(&format!(" #{}", note.tags.join(" #")));
        }
        println!("{}", line);
    }

    println!("{} notes.", notes.len());
    Ok(())
}

fn cmd_view(id_str: &str) -> Result<(), String> {
    let id = parse_id(id_str)?;
    let note = get_note(id)?;
    println!("[{}] {}", note.id, note.text);

    if let Ok(created) = DateTime::parse_from_rfc3339(&note.created_at) {
        println!(
            "    Created: {}",
            created.with_timezone(&Utc).format("%Y-%m-%d")
        );
    }
    if !note.updated_at.is_empty()
        && let Ok(updated) = DateTime::parse_from_rfc3339(&note.updated_at)
    {
        println!(
            "    Updated: {}",
            updated.with_timezone(&Utc).format("%Y-%m-%d")
        );
    }
    if !note.tags.is_empty() {
        println!("    Tags: #{}", note.tags.join(" #"));
    }

    Ok(())
}

fn cmd_done(id_strs: &[String], force: bool) -> Result<(), String> {
    let ids = id_strs
        .iter()
        .map(|s| parse_id(s))
        .collect::<Result<Vec<_>, _>>()?;

    let removed = remove_notes(&ids, force)?;
    for note in removed {
        println!("Removed [{}] {}", note.id, note.text);
    }

    Ok(())
}

fn cmd_edit(id_str: &str, text: String) -> Result<(), String> {
    let id = parse_id(id_str)?;
    let note = edit_note(id, &text)?;
    println!("Updated [{}] {}", note.id, note.text);
    Ok(())
}

fn cmd_tag(id_str: &str, tags: &[String]) -> Result<(), String> {
    let id = parse_id(id_str)?;
    let note = tag_note(id, tags)?;
    println!(
        "Tagged [{}] {}: #{}",
        note.id,
        note.text,
        note.tags.join(" #")
    );
    Ok(())
}

fn cmd_untag(id_str: &str, tag: &str) -> Result<(), String> {
    let id = parse_id(id_str)?;
    let note = untag_note(id, tag)?;
    println!("Removed tag #{} from [{}] {}", tag, note.id, note.text);
    Ok(())
}

fn cmd_tags() -> Result<(), String> {
    let notes = load_notes()?;
    let counts = collect_tags(&notes);
    if counts.is_empty() {
        println!("No tags.");
        return Ok(());
    }

    let sorted: BTreeMap<String, usize> = counts.into_iter().collect();
    for (tag, count) in sorted {
        println!("{:<20} {}", tag, count);
    }

    Ok(())
}

fn cmd_append(id_str: &str, text: String) -> Result<(), String> {
    let id = parse_id(id_str)?;
    let note = append_note(id, &text)?;
    println!("Updated [{}] {}", note.id, note.text);
    Ok(())
}

fn cmd_clear(force: bool) -> Result<(), String> {
    let notes = load_notes()?;
    if notes.is_empty() {
        println!("No notes.");
        return Ok(());
    }

    if !force {
        print!("Remove all {} notes? [y/N] ", notes.len());
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.trim().to_lowercase() != "y" {
            return Ok(());
        }
    }

    clear_notes()?;
    println!("Cleared.");
    Ok(())
}

fn cmd_search(query: &str) -> Result<(), String> {
    let results = search_notes(query)?;
    if results.is_empty() {
        println!("No matches.");
        return Ok(());
    }

    let color = stdout_is_terminal();
    for note in &results {
        if color {
            println!("[{}] {}", note.id, highlight_match(&note.text, query));
        } else {
            println!("[{}] {}", note.id, note.text);
        }
    }
    println!("{} matches.", results.len());
    Ok(())
}

fn cmd_export() -> Result<(), String> {
    let notes = load_notes()?;
    for note in notes {
        println!(
            "{}",
            serde_json::to_string(&note).map_err(|e| e.to_string())?
        );
    }
    Ok(())
}

fn cmd_import<R: Read>(reader: R) -> Result<(), String> {
    let mut incoming = Vec::<Note>::new();
    let br = io::BufReader::new(reader);

    for (idx, line) in br.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let note: Note = serde_json::from_str(trimmed)
            .map_err(|e| format!("line {}: invalid JSON: {}", idx + 1, e))?;
        if note.text.trim().is_empty() {
            return Err(format!("line {}: note text cannot be empty", idx + 1));
        }
        if note.created_at.is_empty() || DateTime::parse_from_rfc3339(&note.created_at).is_err() {
            return Err(format!("line {}: invalid created_at timestamp", idx + 1));
        }
        if !note.updated_at.is_empty() && DateTime::parse_from_rfc3339(&note.updated_at).is_err() {
            return Err(format!("line {}: invalid updated_at timestamp", idx + 1));
        }
        incoming.push(note);
    }

    if incoming.is_empty() {
        println!("No notes to import.");
        return Ok(());
    }

    let count = incoming.len();
    import_notes(incoming)?;
    println!("Imported {} notes.", count);
    Ok(())
}

fn cmd_lock() -> Result<(), String> {
    let notes = if notes_file_is_encrypted() {
        let current = prompt_password("Current password: ")?;
        set_active_password(current);
        match load_notes() {
            Ok(v) => v,
            Err(e) => {
                set_active_password(String::new());
                return Err(e);
            }
        }
    } else {
        load_notes()?
    };

    let pw = prompt_password("New password: ")?;
    if pw.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    let confirm = prompt_password("Confirm password: ")?;
    if pw != confirm {
        return Err("passwords do not match".to_string());
    }

    set_active_password(pw);
    scriv::save_notes(&notes)?;
    println!("Notes are now password protected.");
    Ok(())
}

fn cmd_unlock() -> Result<(), String> {
    if !notes_file_is_encrypted() {
        println!("Notes are not password protected.");
        return Ok(());
    }

    let pw = prompt_password("Password: ")?;
    set_active_password(pw);
    let notes = match load_notes() {
        Ok(v) => v,
        Err(e) => {
            set_active_password(String::new());
            return Err(e);
        }
    };
    set_active_password(String::new());
    scriv::save_notes(&notes)?;
    println!("Password protection removed.");
    Ok(())
}

/// Parse args, dispatch commands, and normalize user-facing errors.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let cmd = &args[1];
    let no_prompt = [
        "lock",
        "unlock",
        "-h",
        "--help",
        "help",
        "-V",
        "--version",
        "version",
    ]
    .contains(&cmd.as_str());

    if notes_file_is_encrypted() && !no_prompt {
        match prompt_password("Password: ") {
            Ok(pw) => set_active_password(pw),
            Err(e) => fatal(&format!("cannot read password: {}", e)),
        }
    }

    let result = match cmd.as_str() {
        "add" => {
            let text = text_from_stdin_or_args(&args, 2)
                .unwrap_or_else(|_| fatal("usage: scriv add <text>"));
            cmd_add(text)
        }
        "list" => {
            let mut opts = ListOptions::default();
            for arg in &args[2..] {
                if let Some(value) = arg.strip_prefix("--tag=") {
                    opts.tag = value.to_string();
                } else if let Some(value) = arg.strip_prefix("--sort=") {
                    opts.sort = value.to_string();
                } else if let Some(value) = arg.strip_prefix("--limit=") {
                    let parsed = value.parse::<usize>();
                    match parsed {
                        Ok(v) if v >= 1 => opts.limit = v,
                        _ => fatal("--limit must be a positive integer"),
                    }
                } else if arg == "--full" {
                    opts.full = true;
                } else {
                    fatal(&format!("unknown flag: {}", arg));
                }
            }
            cmd_list(opts)
        }
        "edit" => {
            if args.len() < 3 {
                fatal("usage: scriv edit <id> <text>");
            }
            let text = text_from_stdin_or_args(&args, 3)
                .unwrap_or_else(|_| fatal("usage: scriv edit <id> <text>"));
            cmd_edit(&args[2], text)
        }
        "done" => {
            let mut force = false;
            let mut id_args = Vec::new();
            for arg in &args[2..] {
                if arg == "--force" {
                    force = true;
                } else {
                    id_args.push(arg.clone());
                }
            }
            if id_args.is_empty() {
                fatal("usage: scriv done [--force] <id> [id2...]");
            }
            cmd_done(&id_args, force)
        }
        "search" => {
            if args.len() < 3 {
                fatal("usage: scriv search <query>");
            }
            cmd_search(&args[2..].join(" "))
        }
        "view" => {
            if args.len() < 3 {
                fatal("usage: scriv view <id>");
            }
            cmd_view(&args[2])
        }
        "tag" => {
            if args.len() < 4 {
                fatal("usage: scriv tag <id> <tag1> [tag2...]");
            }
            cmd_tag(&args[2], &args[3..])
        }
        "untag" => {
            if args.len() < 4 {
                fatal("usage: scriv untag <id> <tag>");
            }
            cmd_untag(&args[2], &args[3])
        }
        "tags" => cmd_tags(),
        "append" => {
            if args.len() < 4 {
                fatal("usage: scriv append <id> <text>");
            }
            cmd_append(&args[2], args[3..].join(" "))
        }
        "export" => cmd_export(),
        "import" => cmd_import(io::stdin()),
        "clear" => {
            let force = args.get(2).map(|v| v == "--force").unwrap_or(false);
            cmd_clear(force)
        }
        "lock" => cmd_lock(),
        "unlock" => cmd_unlock(),
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(())
        }
        "-V" | "--version" | "version" => {
            println!("scriv {}", app_version());
            Ok(())
        }
        _ => fatal(&format!(
            "unknown command: {}\nRun 'scriv --help' for usage.",
            cmd
        )),
    };

    if let Err(err) = result {
        if has_active_password() {
            set_active_password(String::new());
        }
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
