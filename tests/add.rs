//! Binary tests of `notes add`.

mod common;

use std::fs;

use common::{TestEnv, read_json};
use serde_json::json;

#[test]
fn add_saves_a_note_quietly() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "Buy milk"])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(
        read_json(&env.notebook()),
        json!({
            "version": 1,
            "notes": [
                { "text": "Buy milk", "tags": [], "created": "2026-11-15T21:00:00Z" }
            ]
        })
    );
}

#[test]
fn second_add_appends_in_insertion_order() {
    let env = TestEnv::new();

    env.notes().args(["add", "Buy milk"]).assert().success();
    env.notes()
        .env("NOTES_NOW", "2026-11-15T20:00:00Z")
        .args(["add", "Call mum"])
        .assert()
        .success();

    assert_eq!(
        read_json(&env.notebook()),
        json!({
            "version": 1,
            "notes": [
                { "text": "Buy milk", "tags": [], "created": "2026-11-15T21:00:00Z" },
                { "text": "Call mum", "tags": [], "created": "2026-11-15T20:00:00Z" }
            ]
        })
    );
}

#[test]
fn created_is_stored_in_utc() {
    let env = TestEnv::new();

    env.notes()
        .env("NOTES_NOW", "2026-11-15T22:00:00+01:00")
        .args(["add", "Buy milk"])
        .assert()
        .success();

    assert_eq!(
        read_json(&env.notebook())["notes"][0]["created"],
        json!("2026-11-15T21:00:00Z")
    );
}

#[test]
fn invalid_notes_now_is_an_error_and_creates_nothing() {
    let env = TestEnv::new();

    env.notes()
        .env("NOTES_NOW", "xyz")
        .args(["add", "Buy milk"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("error: invalid NOTES_NOW 'xyz': expected an RFC 3339 instant\n");

    assert!(!env.notebook().exists());
}

#[test]
fn add_creates_missing_parent_directories() {
    let env = TestEnv::new();
    let notebook = env.dir().join("a").join("b").join("my-notes.json");

    env.notes()
        .env("NOTES_NOTEBOOK", &notebook)
        .args(["add", "Buy milk"])
        .assert()
        .success();

    assert_eq!(read_json(&notebook)["notes"][0]["text"], json!("Buy milk"));
}

#[test]
fn add_leaves_no_temporary_file() {
    let env = TestEnv::new();
    let dir = env.dir().join("nb");
    let notebook = dir.join("notebook.json");

    env.notes()
        .env("NOTES_NOTEBOOK", &notebook)
        .args(["add", "Buy milk"])
        .assert()
        .success();
    env.notes()
        .env("NOTES_NOTEBOOK", &notebook)
        .args(["add", "Call mum"])
        .assert()
        .success();

    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec!["notebook.json"]);
}

#[test]
fn text_starting_with_a_dash_follows_double_dash() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "--", "-5 degrees"])
        .assert()
        .success();

    assert_eq!(
        read_json(&env.notebook())["notes"][0]["text"],
        json!("-5 degrees")
    );
}

#[test]
fn text_starting_with_a_dash_without_double_dash_is_a_usage_error() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "-5 degrees"])
        .assert()
        .code(2)
        .stdout("");

    assert!(!env.notebook().exists());
}

#[test]
fn add_without_text_is_a_usage_error() {
    let env = TestEnv::new();

    env.notes().arg("add").assert().code(2).stdout("");

    assert!(!env.notebook().exists());
}

#[test]
fn add_help_shows_text_and_tag() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("<TEXT>"))
        .stdout(predicates::str::contains("-t, --tag <TAG>"));
}

const INVALID_TAG_HINT: &str =
    "use 1-32 letters, digits, '-' or '_', starting with a letter or digit";

fn stored_tags(env: &TestEnv) -> serde_json::Value {
    read_json(&env.notebook())["notes"][0]["tags"].clone()
}

#[test]
fn tags_are_lowercased_and_deduplicated() {
    let env = TestEnv::new();

    env.notes()
        .args([
            "add", "Buy milk", "-t", "Home", "-t", "shopping", "-t", "HOME",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(stored_tags(&env), json!(["home", "shopping"]));
}

#[test]
fn duplicate_tags_keep_the_first_occurrence_order() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "x", "-t", "b", "-t", "a", "-t", "B"])
        .assert()
        .success();

    assert_eq!(stored_tags(&env), json!(["b", "a"]));
}

#[test]
fn no_tags_stores_an_empty_array() {
    let env = TestEnv::new();

    env.notes().args(["add", "Call Anna"]).assert().success();

    assert_eq!(stored_tags(&env), json!([]));
}

#[test]
fn valid_tag_characters_and_maximum_length_are_accepted() {
    let env = TestEnv::new();
    let long = "A".repeat(32);

    env.notes()
        .args(["add", "x", "-t", "a-b_9", "-t", &long])
        .assert()
        .success();

    assert_eq!(stored_tags(&env), json!(["a-b_9", "a".repeat(32)]));
}

#[test]
fn invalid_tags_are_rejected_and_nothing_is_saved() {
    let too_long = "a".repeat(33);
    for tag in ["caffè", "", too_long.as_str(), "_x", "a b"] {
        let env = TestEnv::new();

        env.notes()
            .args(["add", "x", "-t", tag])
            .assert()
            .code(1)
            .stdout("")
            .stderr(format!("error: invalid tag '{tag}': {INVALID_TAG_HINT}\n"));

        assert!(!env.notebook().exists());
    }
}

#[test]
fn invalid_tag_leaves_an_existing_notebook_unchanged() {
    let env = TestEnv::new();
    env.notes().args(["add", "first"]).assert().success();
    let before = fs::read(env.notebook()).unwrap();

    env.notes()
        .args(["add", "x", "-t", "caffè"])
        .assert()
        .code(1);

    assert_eq!(fs::read(env.notebook()).unwrap(), before);
}

#[test]
fn the_first_invalid_tag_is_reported() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "x", "-t", "ok", "-t", "b@d", "-t", "c!"])
        .assert()
        .code(1)
        .stderr(format!("error: invalid tag 'b@d': {INVALID_TAG_HINT}\n"));

    assert!(!env.notebook().exists());
}

#[test]
fn an_invalid_tag_is_reported_before_an_invalid_clock() {
    let env = TestEnv::new();

    env.notes()
        .env("NOTES_NOW", "xyz")
        .args(["add", "x", "-t", "caffè"])
        .assert()
        .code(1)
        .stderr(format!("error: invalid tag 'caffè': {INVALID_TAG_HINT}\n"));
}
