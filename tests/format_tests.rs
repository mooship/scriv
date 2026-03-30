//! Tests for pure formatting helpers in src/format.rs.

use chrono::{Duration, Utc};
use scriv::{highlight_match, note_age, read_stdin_text};
use std::io::Cursor;

#[test]
fn highlight_match_empty_query_returns_unchanged() {
    let result = highlight_match("hello world", "");
    assert_eq!(result, "hello world");
}

#[test]
fn highlight_match_wraps_match_in_ansi() {
    let result = highlight_match("Fix the Auth Bug", "auth");
    assert!(result.contains("\x1b[1;33mAuth\x1b[0m"));
    assert!(result.contains("Fix the "));
    assert!(result.contains(" Bug"));
}

#[test]
fn highlight_match_multiple_occurrences() {
    let result = highlight_match("foo bar foo", "foo");
    assert_eq!(result.matches("\x1b[1;33m").count(), 2);
}

#[test]
fn highlight_match_no_match_returns_unchanged() {
    let result = highlight_match("hello world", "xyz");
    assert_eq!(result, "hello world");
}

#[test]
fn note_age_invalid_timestamp_returns_question_mark() {
    assert_eq!(note_age("not-a-date"), "?");
    assert_eq!(note_age(""), "?");
}

#[test]
fn note_age_recent_returns_lt1h() {
    let ts = (Utc::now() - Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "<1h");
}

#[test]
fn note_age_hours() {
    let ts = (Utc::now() - Duration::hours(3))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "3h");
}

#[test]
fn note_age_days() {
    let ts = (Utc::now() - Duration::days(5))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "5d");
}

#[test]
fn note_age_weeks() {
    let ts = (Utc::now() - Duration::days(21))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "3w");
}

#[test]
fn note_age_months() {
    let ts = (Utc::now() - Duration::days(90))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "3mo");
}

#[test]
fn note_age_years() {
    let ts = (Utc::now() - Duration::days(730))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(note_age(&ts), "2y");
}

#[test]
fn read_stdin_text_returns_trimmed_text() {
    let input = Cursor::new(b"hello\nworld\n");
    let result = read_stdin_text(input).expect("read stdin");
    assert_eq!(result, "hello\nworld");
}

#[test]
fn read_stdin_text_empty_returns_error() {
    let input = Cursor::new(b"");
    let err = read_stdin_text(input).expect_err("expected error");
    assert_eq!(err, "no text provided via stdin");
}

#[test]
fn read_stdin_text_whitespace_only_returns_error() {
    let input = Cursor::new(b"   \n  \n");
    let err = read_stdin_text(input).expect_err("expected error");
    assert_eq!(err, "no text provided via stdin");
}

#[test]
fn read_stdin_text_strips_crlf() {
    let input = Cursor::new(b"hello\r\nworld\r\n");
    let result = read_stdin_text(input).expect("read stdin");
    assert_eq!(result, "hello\nworld");
}

#[test]
fn highlight_match_unicode_case_fold_no_panic() {
    let result = highlight_match("Ökologie test", "ökologie");
    assert!(result.contains("\x1b[1;33m"));
    assert!(result.contains("Ökologie"));
}

#[test]
fn highlight_match_expansion_char_no_panic() {
    let result = highlight_match("İstanbul", "i");
    assert!(!result.is_empty());
}
