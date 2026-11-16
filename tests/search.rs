//! Binary tests of `notes search`.

mod common;

use std::fs;

use common::TestEnv;
use serde_json::json;

fn write_notebook(env: &TestEnv, notes: serde_json::Value) {
    let content = json!({ "version": 1, "notes": notes });
    fs::write(
        env.notebook(),
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .unwrap();
}

fn three_notes(env: &TestEnv) {
    write_notebook(
        env,
        json!([
            { "text": "Call Anna", "tags": [], "created": "2026-11-15T21:00:00Z" },
            { "text": "Buy milk", "tags": ["home", "shopping"], "created": "2026-11-15T22:00:00Z" },
            { "text": "first\nsecond", "tags": ["work"], "created": "2026-11-14T09:05:00Z" }
        ]),
    );
}

#[test]
fn search_prints_the_notes_with_the_tag_ignoring_capitalisation() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes()
        .args(["search", "HOME"])
        .assert()
        .success()
        .stdout("2026-11-15 22:00  Buy milk  [home, shopping]\n")
        .stderr("");
}

#[test]
fn search_prints_several_matches_in_the_layout_and_order_of_list() {
    let env = TestEnv::new();
    write_notebook(
        &env,
        json!([
            { "text": "old\nlines", "tags": ["work"], "created": "2026-11-14T09:05:00Z" },
            { "text": "Call Anna", "tags": [], "created": "2026-11-15T21:00:00Z" },
            { "text": "standup", "tags": ["work", "team"], "created": "2026-11-15T10:00:00Z" },
            { "text": "tie, added later", "tags": ["work"], "created": "2026-11-14T09:05:00Z" }
        ]),
    );

    let expected = [
        "2026-11-15 10:00  standup  [work, team]",
        "2026-11-14 09:05  tie, added later  [work]",
        "2026-11-14 09:05  old",
        "                  lines  [work]",
        "",
    ]
    .join("\n");
    env.notes()
        .args(["search", "work"])
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn search_without_a_match_says_so_on_stderr_and_succeeds() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes()
        .args(["search", "Travel"])
        .assert()
        .success()
        .stdout("")
        .stderr("No notes tagged travel.\n");
}

#[test]
fn search_of_a_missing_notebook_says_no_match_and_creates_nothing() {
    let env = TestEnv::new();

    env.notes()
        .args(["search", "home"])
        .assert()
        .success()
        .stdout("")
        .stderr("No notes tagged home.\n");

    assert!(!env.notebook().exists());
}

#[test]
fn search_never_matches_a_note_without_tags() {
    let env = TestEnv::new();
    write_notebook(
        &env,
        json!([
            { "text": "Call Anna", "tags": [], "created": "2026-11-15T21:00:00Z" }
        ]),
    );

    env.notes()
        .args(["search", "call"])
        .assert()
        .success()
        .stdout("")
        .stderr("No notes tagged call.\n");
}

#[test]
fn search_rejects_an_invalid_tag_without_reading_the_notebook() {
    let env = TestEnv::new();
    fs::write(env.notebook(), "not json").unwrap();

    env.notes()
        .args(["search", "two_words!"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(
            "error: invalid tag 'two_words!': use 1-32 letters, digits, '-' or '_', starting with a letter or digit\n",
        );
}
