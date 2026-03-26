//! Integration tests for core note behavior and compatibility guarantees.

mod common;
use common::{TestEnv, lock_test};
use scriv::*;
use std::fs;

#[test]
fn add_note_assigns_id_1_when_empty() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let note = add_note("first").expect("add note");
    assert_eq!(note.id, 1);
}

#[test]
fn add_note_id_is_max_plus_one() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("one").expect("add one");
    add_note("two").expect("add two");
    let note = add_note("three").expect("add three");
    assert_eq!(note.id, 3);
}

#[test]
fn remove_note_ids_are_stable() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("one").expect("add one");
    add_note("two").expect("add two");
    add_note("three").expect("add three");
    remove_note(2).expect("remove two");

    let note = add_note("four").expect("add four");
    assert_eq!(note.id, 4);
}

#[test]
fn remove_note_missing_id_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let err = remove_note(99).expect_err("expected error");
    assert_eq!(err, "no note with id 99");
}

#[test]
fn search_notes_case_insensitive() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("Fix the Auth Bug").expect("add 1");
    add_note("write tests").expect("add 2");

    let results = search_notes("auth").expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn search_notes_matches_tags() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("buy groceries").expect("add 1");
    tag_note(1, &["work".to_string()]).expect("tag 1");
    add_note("standup notes").expect("add 2");

    let results = search_notes("work").expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn edit_note_sets_updated_at() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("original").expect("add");
    let updated = edit_note(1, "revised").expect("edit");

    assert_eq!(updated.text, "revised");
    assert!(!updated.updated_at.is_empty());
}

#[test]
fn edit_note_missing_id_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let err = edit_note(99, "text").expect_err("expected error");
    assert_eq!(err, "no note with id 99");
}

#[test]
fn get_note_returns_correct_note() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("hello world").expect("add");
    let note = get_note(1).expect("get note");
    assert_eq!(note.id, 1);
    assert_eq!(note.text, "hello world");
}

#[test]
fn get_note_missing_id_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let err = get_note(99).expect_err("expected error");
    assert_eq!(err, "no note with id 99");
}

#[test]
fn append_note_concatenates_text() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("hello").expect("add");
    let note = append_note(1, "world").expect("append");

    assert_eq!(note.text, "hello world");
    assert!(!note.updated_at.is_empty());

    let persisted = get_note(1).expect("get note");
    assert_eq!(persisted.text, "hello world");
}

#[test]
fn append_note_missing_id_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let err = append_note(99, "text").expect_err("expected error");
    assert_eq!(err, "no note with id 99");
}

#[test]
fn tag_note_deduplicates_tags() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("buy milk").expect("add");
    tag_note(1, &["groceries".to_string()]).expect("tag 1");
    let note = tag_note(1, &["groceries".to_string()]).expect("tag 2");
    assert_eq!(note.tags.len(), 1);

    let persisted = load_notes().expect("load notes");
    assert_eq!(persisted[0].tags.len(), 1);
}

#[test]
fn untag_note_removes_tag() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("task").expect("add");
    tag_note(1, &["work".to_string()]).expect("tag");
    let note = untag_note(1, "work").expect("untag");
    assert!(note.tags.is_empty());
}

#[test]
fn untag_note_noop_when_tag_absent() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("task").expect("add");
    tag_note(1, &["work".to_string()]).expect("tag");
    let note = untag_note(1, "nonexistent").expect("untag nonexistent");
    assert_eq!(note.tags, vec!["work".to_string()]);
}

#[test]
fn untag_note_missing_id_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let err = untag_note(99, "tag").expect_err("expected error");
    assert_eq!(err, "no note with id 99");
}

#[test]
fn clear_notes_empties_store() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("one").expect("add 1");
    add_note("two").expect("add 2");
    add_note("three").expect("add 3");
    clear_notes().expect("clear notes");

    let notes = load_notes().expect("load notes");
    assert!(notes.is_empty());
}

#[test]
fn collect_tags_counts_correctly() {
    let notes = vec![
        Note {
            id: 1,
            text: "a".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            tags: vec!["work".to_string(), "urgent".to_string()],
        },
        Note {
            id: 2,
            text: "b".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            tags: vec!["work".to_string()],
        },
        Note {
            id: 3,
            text: "c".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            tags: vec![],
        },
    ];

    let counts = collect_tags(&notes);
    assert_eq!(counts["work"], 2);
    assert_eq!(counts["urgent"], 1);
    assert!(!counts.contains_key("nonexistent"));
}

#[test]
fn list_notes_sort_by_updated() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        concat!(
            "{\"id\":1,\"text\":\"oldest\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n",
            "{\"id\":2,\"text\":\"edited\",\"created_at\":\"2024-01-02T00:00:00Z\",\"updated_at\":\"2024-12-01T00:00:00Z\"}\n",
            "{\"id\":3,\"text\":\"newer\",\"created_at\":\"2024-06-01T00:00:00Z\"}\n",
        ),
    )
    .expect("write fixture");

    let notes = list_notes(&ListOptions {
        sort: "updated".to_string(),
        ..Default::default()
    })
    .expect("list notes");

    assert_eq!(notes[0].id, 2);
    assert_eq!(notes[1].id, 3);
    assert_eq!(notes[2].id, 1);
}

#[test]
fn list_notes_sort_by_date() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        concat!(
            "{\"id\":1,\"text\":\"oldest\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n",
            "{\"id\":2,\"text\":\"middle\",\"created_at\":\"2024-06-01T00:00:00Z\"}\n",
            "{\"id\":3,\"text\":\"newest\",\"created_at\":\"2024-12-01T00:00:00Z\"}\n",
        ),
    )
    .expect("write fixture");

    let notes = list_notes(&ListOptions {
        sort: "date".to_string(),
        ..Default::default()
    })
    .expect("list notes");

    assert_eq!(notes[0].id, 3);
    assert_eq!(notes[1].id, 2);
    assert_eq!(notes[2].id, 1);
}

#[test]
fn list_notes_sort_unknown_returns_error() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("note").expect("add");
    let err = list_notes(&ListOptions {
        sort: "bogus".to_string(),
        ..Default::default()
    })
    .expect_err("expected unknown sort error");

    assert!(err.contains("unknown sort"));
}

#[test]
fn list_notes_tag_filter() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("work note").expect("add 1");
    tag_note(1, &["Work".to_string()]).expect("tag 1");
    add_note("personal note").expect("add 2");
    tag_note(2, &["personal".to_string()]).expect("tag 2");
    add_note("untagged note").expect("add 3");

    let notes = list_notes(&ListOptions {
        tag: "work".to_string(),
        ..Default::default()
    })
    .expect("list notes");

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, 1);
}

#[test]
fn list_notes_limit_truncates() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    for i in 1..=5 {
        add_note(&format!("note {}", i)).expect("add note");
    }

    let notes = list_notes(&ListOptions {
        limit: 3,
        ..Default::default()
    })
    .expect("list notes");

    assert_eq!(notes.len(), 3);
}

#[test]
fn remove_notes_force_ignores_missing() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("alpha").expect("add alpha");
    add_note("beta").expect("add beta");

    let removed = remove_notes(&[1, 99], true).expect("remove notes");
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].id, 1);
}

#[test]
fn remove_notes_non_force_all_or_nothing() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("alpha").expect("add alpha");
    add_note("beta").expect("add beta");

    let err = remove_notes(&[1, 99], false).expect_err("expected error");
    assert!(err.contains("no note with id 99"));
    assert!(err.contains("no notes were removed"));

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 2);
}

#[test]
fn import_notes_assigns_new_ids() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    add_note("existing").expect("add existing");
    import_notes(vec![
        Note {
            id: 1,
            text: "imported one".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
        Note {
            id: 2,
            text: "imported two".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
    ])
    .expect("import notes");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 3);
    assert_eq!(notes[1].id, 2);
    assert_eq!(notes[2].id, 3);
}

#[test]
fn compat_minimal_fixture_loads() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        "{\"id\":1,\"text\":\"hello world\",\"created_at\":\"2024-01-15T10:30:00Z\"}",
    )
    .expect("write fixture");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, 1);
    assert_eq!(notes[0].updated_at, "");
    assert!(notes[0].tags.is_empty());
}

#[test]
fn compat_unknown_fields_are_ignored() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        "{\"id\":1,\"text\":\"note\",\"created_at\":\"2024-01-01T00:00:00Z\",\"future_field\":\"ignored\",\"another\":123}",
    )
    .expect("write fixture");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "note");
}

#[test]
fn save_load_encrypted_notes() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    set_active_password("testpassword".to_string());

    let want = vec![
        Note {
            id: 1,
            text: "secret note".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
        Note {
            id: 2,
            text: "another secret".to_string(),
            created_at: "2024-02-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: vec!["private".to_string()],
        },
    ];

    save_notes(&want).expect("save notes");
    let raw = fs::read(notes_path()).expect("read raw");
    assert!(is_encrypted_data(&raw));

    let got = load_notes().expect("load notes");
    assert_eq!(got.len(), want.len());
    assert_eq!(got[0].text, "secret note");
}
