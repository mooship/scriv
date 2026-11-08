//! End-to-end tests that run the compiled `scriv` binary.
//!
//! Each test gets its own data directory and runs the binary as a separate
//! process, so none of them touch this process's storage globals or need
//! `lock_test()`.

use scriv::Note;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const OLDEST: &str = "2024-01-01T00:00:00Z";
const MIDDLE: &str = "2024-02-01T00:00:00Z";
const NEWEST: &str = "2024-03-01T00:00:00Z";

/// A throwaway data directory plus a way to run the binary against it.
struct Cli {
    root: TempDir,
}

impl Cli {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("tempdir"),
        }
    }

    /// Where the binary keeps notes when `HOME`, `XDG_DATA_HOME` and
    /// `APPDATA` all point at the temp directory.
    fn notes_file(&self) -> PathBuf {
        let root = self.root.path();
        if cfg!(target_os = "macos") {
            root.join("Library")
                .join("Application Support")
                .join("scriv")
                .join("notes.json")
        } else {
            root.join("scriv").join("notes.json")
        }
    }

    fn command(&self) -> Command {
        let root = self.root.path();
        let mut command = Command::new(env!("CARGO_BIN_EXE_scriv"));
        command
            .env("HOME", root)
            .env("XDG_DATA_HOME", root)
            .env("APPDATA", root)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        detach_from_terminal(&mut command);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with_stdin(args, "")
    }

    fn run_with_stdin(&self, args: &[&str], stdin: impl AsRef<[u8]>) -> Output {
        run_command(self.command().args(args), stdin.as_ref())
    }

    /// Write `notes` straight into the store, bypassing the binary.
    fn seed(&self, notes: &[Note]) {
        let path = self.notes_file();
        fs::create_dir_all(path.parent().expect("notes dir")).expect("create data dir");
        let mut ndjson = String::new();
        for note in notes {
            ndjson.push_str(&serde_json::to_string(note).expect("serialise note"));
            ndjson.push('\n');
        }
        fs::write(path, ndjson).expect("seed notes");
    }

    /// Add a note through the binary and return its output line.
    fn add(&self, text: &str) -> String {
        succeeded(&self.run_with_stdin(&["add"], text))
    }
}

fn run_command(command: &mut Command, stdin: &[u8]) -> Output {
    let mut child = command.spawn().expect("spawn scriv");
    let mut pipe = child.stdin.take().expect("child stdin");
    match pipe.write_all(stdin) {
        Err(e) if e.kind() != ErrorKind::BrokenPipe => panic!("write stdin: {e}"),
        _ => {}
    }
    drop(pipe);
    child.wait_with_output().expect("wait for scriv")
}

/// Start the child in its own session so `/dev/tty` cannot be opened. Without
/// this, password prompts would read the developer's real terminal.
#[cfg(unix)]
fn detach_from_terminal(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe extern "C" {
        fn setsid() -> i32;
    }

    fn start_new_session() -> std::io::Result<()> {
        unsafe { setsid() };
        Ok(())
    }

    // SAFETY: `setsid` is async-signal-safe, which is all `pre_exec` requires.
    unsafe { command.pre_exec(start_new_session) };
}

#[cfg(not(unix))]
fn detach_from_terminal(_command: &mut Command) {}

/// Assert a zero exit status and return stdout.
fn succeeded(out: &Output) -> String {
    assert!(
        out.status.success(),
        "scriv failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Assert exit status 1 and return stderr.
fn failed(out: &Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit status 1, stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn note(id: u64, text: &str, created_at: &str) -> Note {
    Note {
        id,
        text: text.to_string(),
        created_at: created_at.to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }
}

/// The ids in a `list` output, in the order printed.
fn listed_ids(listing: &str) -> Vec<&str> {
    listing
        .lines()
        .filter(|line| line.starts_with('['))
        .filter_map(|line| line.split(' ').next())
        .collect()
}

#[test]
fn no_arguments_prints_usage_and_fails() {
    let cli = Cli::new();
    let out = cli.run(&[]);

    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: scriv <command> [arguments]"));
}

#[test]
fn help_flags_print_usage_with_the_version() {
    let cli = Cli::new();
    for flag in ["-h", "--help", "help"] {
        let text = succeeded(&cli.run(&[flag]));
        assert!(text.contains(&format!("Version: {}", env!("CARGO_PKG_VERSION"))));
        assert!(text.contains("Usage: scriv <command> [arguments]"));
    }
}

#[test]
fn version_flags_print_the_crate_version() {
    let cli = Cli::new();
    for flag in ["-V", "--version", "version"] {
        assert_eq!(
            succeeded(&cli.run(&[flag])),
            format!("scriv {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn unknown_command_fails_and_points_at_help() {
    let cli = Cli::new();
    let err = failed(&cli.run(&["frobnicate"]));

    assert!(err.contains("Error: unknown command: frobnicate"));
    assert!(err.contains("Run 'scriv --help' for usage."));
}

#[test]
fn add_reads_text_from_stdin_and_stores_it() {
    let cli = Cli::new();

    assert_eq!(cli.add("buy oat milk\n"), "Added [1] buy oat milk\n");
    assert!(cli.notes_file().is_file());
    assert_eq!(cli.add("second"), "Added [2] second\n");
}

#[test]
fn add_without_text_fails_with_usage() {
    let cli = Cli::new();
    let err = failed(&cli.run(&["add"]));

    assert!(err.contains("Error: usage: scriv add <text>"));
}

#[test]
fn add_strips_terminal_escapes_from_its_output() {
    let cli = Cli::new();

    assert_eq!(cli.add("red\u{1b}[31m alert"), "Added [1] red[31m alert\n");
}

#[test]
fn list_with_no_notes_says_so() {
    let cli = Cli::new();

    assert_eq!(succeeded(&cli.run(&["list"])), "No notes.\n");
}

#[test]
fn list_shows_age_tags_and_a_count() {
    let cli = Cli::new();
    cli.add("first");
    cli.add("second");
    succeeded(&cli.run(&["tag", "1", "work", "urgent"]));

    assert_eq!(
        succeeded(&cli.run(&["list"])),
        "[1] (<1h) first #work #urgent\n[2] (<1h) second\n2 notes.\n"
    );
}

#[test]
fn list_filters_by_tag_and_limits_results() {
    let cli = Cli::new();
    cli.seed(&[
        Note {
            tags: vec!["Work".to_string()],
            ..note(1, "one", OLDEST)
        },
        note(2, "two", MIDDLE),
        Note {
            tags: vec!["work".to_string()],
            ..note(3, "three", NEWEST)
        },
    ]);

    let tagged = succeeded(&cli.run(&["list", "--tag=WORK"]));
    assert_eq!(listed_ids(&tagged), ["[1]", "[3]"]);
    assert!(tagged.ends_with("2 notes.\n"));

    let limited = succeeded(&cli.run(&["list", "--limit=2"]));
    assert_eq!(listed_ids(&limited), ["[1]", "[2]"]);
}

#[test]
fn list_sorts_by_date_and_by_update_time() {
    let cli = Cli::new();
    cli.seed(&[
        note(1, "one", OLDEST),
        note(2, "two", NEWEST),
        Note {
            updated_at: "2024-09-01T00:00:00Z".to_string(),
            ..note(3, "three", MIDDLE)
        },
    ]);

    let by_date = succeeded(&cli.run(&["list", "--sort=date"]));
    assert_eq!(listed_ids(&by_date), ["[2]", "[3]", "[1]"]);

    let by_update = succeeded(&cli.run(&["list", "--sort=updated"]));
    assert_eq!(listed_ids(&by_update), ["[3]", "[2]", "[1]"]);
}

#[test]
fn list_truncates_long_notes_unless_full_is_given() {
    let cli = Cli::new();
    let long = "a".repeat(100);
    cli.seed(&[note(1, &long, OLDEST)]);

    let truncated = succeeded(&cli.run(&["list"]));
    assert!(truncated.contains(&format!("{}...", "a".repeat(72))));
    assert!(!truncated.contains(&"a".repeat(73)));

    let full = succeeded(&cli.run(&["list", "--full"]));
    assert!(full.contains(&long));
    assert!(!full.contains("..."));
}

#[test]
fn list_rejects_bad_flags_and_sort_modes() {
    let cli = Cli::new();

    for limit in ["--limit=0", "--limit=abc"] {
        let err = failed(&cli.run(&["list", limit]));
        assert!(err.contains("--limit must be a positive integer"));
    }
    let err = failed(&cli.run(&["list", "--bogus"]));
    assert!(err.contains("unknown flag: --bogus"));
    let err = failed(&cli.run(&["list", "--sort=nope"]));
    assert!(err.contains("Error: unknown sort \"nope\""));
}

#[test]
fn view_shows_dates_and_tags() {
    let cli = Cli::new();
    cli.seed(&[Note {
        updated_at: "2024-03-14T09:00:00Z".to_string(),
        tags: vec!["work".to_string(), "critical".to_string()],
        ..note(1, "fix the auth bug", "2024-03-12T10:00:00Z")
    }]);

    assert_eq!(
        succeeded(&cli.run(&["view", "1"])),
        "[1] fix the auth bug\n    Created: 2024-03-12\n    Updated: 2024-03-14\n    Tags: #work #critical\n"
    );
}

#[test]
fn view_skips_dates_it_cannot_parse() {
    let cli = Cli::new();
    cli.seed(&[Note {
        updated_at: "later".to_string(),
        ..note(1, "odd dates", "yesterday")
    }]);

    assert_eq!(succeeded(&cli.run(&["view", "1"])), "[1] odd dates\n");
}

#[test]
fn view_rejects_missing_notes_bad_ids_and_missing_arguments() {
    let cli = Cli::new();

    assert!(failed(&cli.run(&["view", "9"])).contains("Error: no note with id 9"));
    assert!(failed(&cli.run(&["view", "abc"])).contains("id must be a positive integer"));
    assert!(failed(&cli.run(&["view"])).contains("usage: scriv view <id>"));
}

#[test]
fn edit_replaces_the_text_read_from_stdin() {
    let cli = Cli::new();
    cli.add("old text");

    assert_eq!(
        succeeded(&cli.run_with_stdin(&["edit", "1"], "new text\n")),
        "Updated [1] new text\n"
    );
}

#[test]
fn edit_rejects_missing_arguments_and_bad_ids() {
    let cli = Cli::new();
    cli.add("old text");

    assert!(failed(&cli.run(&["edit"])).contains("usage: scriv edit <id> <text>"));
    assert!(failed(&cli.run(&["edit", "1"])).contains("usage: scriv edit <id> <text>"));
    let err = failed(&cli.run_with_stdin(&["edit", "zero"], "text"));
    assert!(err.contains("id must be a positive integer"));
}

#[test]
fn append_adds_the_text_read_from_stdin() {
    let cli = Cli::new();
    cli.add("one");

    assert_eq!(
        succeeded(&cli.run_with_stdin(&["append", "1"], "two\n")),
        "Updated [1] one two\n"
    );
}

#[test]
fn append_rejects_missing_arguments() {
    let cli = Cli::new();
    cli.add("one");

    assert!(failed(&cli.run(&["append"])).contains("usage: scriv append <id> <text>"));
    assert!(failed(&cli.run(&["append", "1"])).contains("usage: scriv append <id> <text>"));
}

#[test]
fn done_removes_the_given_notes() {
    let cli = Cli::new();
    cli.seed(&[
        note(1, "one", OLDEST),
        note(2, "two", MIDDLE),
        note(3, "three", NEWEST),
    ]);

    assert_eq!(
        succeeded(&cli.run(&["done", "1", "3"])),
        "Removed [1] one\nRemoved [3] three\n"
    );
    let remaining = succeeded(&cli.run(&["list"]));
    assert_eq!(listed_ids(&remaining), ["[2]"]);
}

#[test]
fn done_fails_on_missing_ids_unless_forced() {
    let cli = Cli::new();
    cli.seed(&[note(1, "one", OLDEST)]);

    let err = failed(&cli.run(&["done", "1", "99"]));
    assert!(err.contains("no note with id 99; no notes were removed"));
    assert_eq!(listed_ids(&succeeded(&cli.run(&["list"]))), ["[1]"]);

    assert_eq!(
        succeeded(&cli.run(&["done", "--force", "1", "99"])),
        "Removed [1] one\n"
    );
}

#[test]
fn done_rejects_missing_arguments_and_bad_ids() {
    let cli = Cli::new();

    assert!(failed(&cli.run(&["done"])).contains("usage: scriv done [--force] <id> [id2...]"));
    assert!(failed(&cli.run(&["done", "--force"])).contains("usage: scriv done"));
    assert!(failed(&cli.run(&["done", "x"])).contains("id must be a positive integer"));
}

#[test]
fn search_prints_matches_without_colour_when_piped() {
    let cli = Cli::new();
    cli.seed(&[
        note(1, "fix the auth bug", OLDEST),
        note(2, "write tests", MIDDLE),
    ]);

    assert_eq!(
        succeeded(&cli.run(&["search", "auth"])),
        "[1] fix the auth bug\n1 matches.\n"
    );
    assert_eq!(
        succeeded(&cli.run(&["search", "write", "tests"])),
        "[2] write tests\n1 matches.\n"
    );
}

#[test]
fn search_reports_no_matches_and_missing_queries() {
    let cli = Cli::new();
    cli.seed(&[note(1, "one", OLDEST)]);

    assert_eq!(succeeded(&cli.run(&["search", "zzz"])), "No matches.\n");
    assert!(failed(&cli.run(&["search"])).contains("usage: scriv search <query>"));
}

#[test]
fn tag_and_untag_update_a_note() {
    let cli = Cli::new();
    cli.add("fix the auth bug");

    assert_eq!(
        succeeded(&cli.run(&["tag", "1", "work", "critical"])),
        "Tagged [1] fix the auth bug: #work #critical\n"
    );
    assert_eq!(
        succeeded(&cli.run(&["untag", "1", "CRITICAL"])),
        "Removed tag #CRITICAL from [1] fix the auth bug\n"
    );
    assert_eq!(
        succeeded(&cli.run(&["untag", "1", "nope"])),
        "Tag #nope not found on [1] fix the auth bug\n"
    );
}

#[test]
fn tag_and_untag_reject_missing_arguments() {
    let cli = Cli::new();
    cli.add("one");

    assert!(failed(&cli.run(&["tag", "1"])).contains("usage: scriv tag <id> <tag1> [tag2...]"));
    assert!(failed(&cli.run(&["untag", "1"])).contains("usage: scriv untag <id> <tag>"));
    assert!(failed(&cli.run(&["tag", "9", "work"])).contains("no note with id 9"));
}

#[test]
fn tags_lists_counts_alphabetically() {
    let cli = Cli::new();
    cli.seed(&[
        Note {
            tags: vec!["work".to_string(), "critical".to_string()],
            ..note(1, "one", OLDEST)
        },
        Note {
            tags: vec!["work".to_string()],
            ..note(2, "two", MIDDLE)
        },
    ]);

    assert_eq!(
        succeeded(&cli.run(&["tags"])),
        format!("{:<20} {}\n{:<20} {}\n", "critical", 1, "work", 2)
    );
}

#[test]
fn tags_with_no_tagged_notes_says_so() {
    let cli = Cli::new();
    cli.add("untagged");

    assert_eq!(succeeded(&cli.run(&["tags"])), "No tags.\n");
}

#[test]
fn export_prints_one_json_object_per_line() {
    let cli = Cli::new();
    let notes = [
        note(1, "one", OLDEST),
        Note {
            tags: vec!["work".to_string()],
            ..note(2, "two", MIDDLE)
        },
    ];
    cli.seed(&notes);

    let expected: String = notes
        .iter()
        .map(|n| format!("{}\n", serde_json::to_string(n).expect("serialise note")))
        .collect();
    assert_eq!(succeeded(&cli.run(&["export"])), expected);
}

#[test]
fn import_appends_notes_with_fresh_ids() {
    let cli = Cli::new();
    cli.add("existing");
    let incoming = concat!(
        "{\"id\":1,\"text\":\"one\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n",
        "{\"id\":1,\"text\":\"two\",\"created_at\":\"2024-01-02T00:00:00Z\"}\n",
    );

    assert_eq!(
        succeeded(&cli.run_with_stdin(&["import"], incoming)),
        "Imported 2 notes.\n"
    );
    let listing = succeeded(&cli.run(&["list"]));
    assert_eq!(listed_ids(&listing), ["[1]", "[2]", "[3]"]);
}

#[test]
fn import_reports_empty_and_invalid_input() {
    let cli = Cli::new();

    assert_eq!(
        succeeded(&cli.run_with_stdin(&["import"], "\n")),
        "No notes to import.\n"
    );
    let err = failed(&cli.run_with_stdin(&["import"], "not json\n"));
    assert!(err.contains("Error: line 1: invalid JSON"));
}

#[test]
fn clear_with_no_notes_says_so() {
    let cli = Cli::new();

    assert_eq!(succeeded(&cli.run(&["clear"])), "No notes.\n");
}

#[test]
fn clear_asks_for_confirmation_and_respects_a_no() {
    let cli = Cli::new();
    cli.add("keep me");

    let declined = succeeded(&cli.run_with_stdin(&["clear"], "n\n"));
    assert_eq!(declined, "Remove all 1 notes? [y/N] ");
    assert_eq!(listed_ids(&succeeded(&cli.run(&["list"]))), ["[1]"]);

    let confirmed = succeeded(&cli.run_with_stdin(&["clear"], "Y\n"));
    assert_eq!(confirmed, "Remove all 1 notes? [y/N] Cleared.\n");
    assert_eq!(succeeded(&cli.run(&["list"])), "No notes.\n");
}

#[test]
fn clear_force_skips_the_confirmation() {
    let cli = Cli::new();
    cli.add("one");
    cli.add("two");

    assert_eq!(succeeded(&cli.run(&["clear", "--force"])), "Cleared.\n");
    assert_eq!(succeeded(&cli.run(&["list"])), "No notes.\n");
}

#[test]
fn unlock_on_a_plain_store_says_nothing_to_do() {
    let cli = Cli::new();
    cli.add("plain");

    assert_eq!(
        succeeded(&cli.run(&["unlock"])),
        "Notes are not password protected.\n"
    );
}

#[cfg(unix)]
mod password_prompts {
    use super::*;

    /// Put an encrypted store in place and return its exact bytes.
    fn seed_encrypted(cli: &Cli) -> Vec<u8> {
        let path = cli.notes_file();
        fs::create_dir_all(path.parent().expect("notes dir")).expect("create data dir");
        let ndjson = serde_json::to_string(&note(1, "secret", OLDEST)).expect("serialise note");
        let encrypted =
            scriv::encrypt_notes(format!("{ndjson}\n").as_bytes(), "pw").expect("encrypt");
        fs::write(&path, &encrypted).expect("seed encrypted notes");
        encrypted
    }

    #[test]
    fn commands_on_an_encrypted_store_fail_when_no_password_can_be_read() {
        let cli = Cli::new();
        let before = seed_encrypted(&cli);

        let err = failed(&cli.run(&["list"]));

        assert!(
            err.contains("Error: cannot read password:"),
            "stderr: {err}"
        );
        assert_eq!(fs::read(cli.notes_file()).expect("read store"), before);
    }

    #[test]
    fn help_and_version_never_prompt_for_a_password() {
        let cli = Cli::new();
        seed_encrypted(&cli);

        assert!(succeeded(&cli.run(&["--help"])).contains("Usage: scriv"));
        assert!(succeeded(&cli.run(&["--version"])).starts_with("scriv "));
    }

    #[test]
    fn lock_and_unlock_fail_cleanly_when_no_password_can_be_read() {
        let cli = Cli::new();
        let before = seed_encrypted(&cli);

        let err = failed(&cli.run(&["lock"]));
        assert!(err.contains("Current password: "), "stderr: {err}");
        let err = failed(&cli.run(&["unlock"]));
        assert!(err.contains("Password: "), "stderr: {err}");

        assert_eq!(fs::read(cli.notes_file()).expect("read store"), before);
    }

    #[test]
    fn lock_on_a_plain_store_fails_without_changing_it() {
        let cli = Cli::new();
        cli.add("plain");
        let before = fs::read(cli.notes_file()).expect("read store");

        let err = failed(&cli.run(&["lock"]));

        assert!(err.contains("New password: "), "stderr: {err}");
        assert_eq!(fs::read(cli.notes_file()).expect("read store"), before);
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn notes_fall_back_to_home_when_xdg_data_home_is_empty() {
    let cli = Cli::new();
    let root = cli.root.path();

    let out = run_command(
        cli.command().env("XDG_DATA_HOME", "").args(["add"]),
        b"fallback",
    );

    assert_eq!(succeeded(&out), "Added [1] fallback\n");
    assert!(
        root.join(".local")
            .join("share")
            .join("scriv")
            .join("notes.json")
            .is_file()
    );
}

#[test]
fn clear_reports_a_confirmation_that_is_not_utf8() {
    let cli = Cli::new();
    cli.add("keep me");

    let err = failed(&cli.run_with_stdin(&["clear"], b"\xff\xfe\n"));

    assert!(err.contains("UTF-8"), "stderr: {err}");
    assert_eq!(listed_ids(&succeeded(&cli.run(&["list"]))), ["[1]"]);
}
