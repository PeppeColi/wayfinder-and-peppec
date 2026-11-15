//! Binary tests of global behaviour: help, `--notebook` and the default location.

mod common;

use common::{TestEnv, read_json};
use predicates::prelude::*;
use predicates::str::contains;
use serde_json::json;

#[test]
fn help_describes_the_tool_and_its_commands() {
    let env = TestEnv::new();

    env.notes()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::starts_with(
            "Take notes from the terminal, tag them, and find them again.",
        ))
        .stdout(predicates::str::is_match(r"(?m)^\s+add\s+Add a note$").unwrap())
        .stdout(
            predicates::str::is_match(r"(?m)^\s+list\s+List notes, most recent first$").unwrap(),
        )
        .stdout(
            predicates::str::is_match(r"(?m)^\s+search\s+List the notes that have a tag$").unwrap(),
        )
        .stdout(predicates::str::is_match(r"(?m)^\s+export\s+Write all notes to a file$").unwrap())
        .stdout(contains("--notebook <PATH>"))
        .stdout(contains("Notebook file"))
        .stdout(contains("[env: NOTES_NOTEBOOK]"))
        .stdout(contains("NOTES_NOW").not());
}

#[test]
fn notebook_flag_wins_over_the_environment() {
    let env = TestEnv::new();

    env.notes()
        .env("NOTES_NOTEBOOK", "b.json")
        .args(["--notebook", "a.json", "add", "Buy milk"])
        .assert()
        .success();

    assert_eq!(
        read_json(&env.dir().join("a.json"))["notes"][0]["text"],
        json!("Buy milk")
    );
    assert!(!env.dir().join("b.json").exists());
}

#[test]
fn notebook_flag_is_accepted_after_the_command() {
    let env = TestEnv::new();

    env.notes()
        .args(["add", "--notebook", "a.json", "Buy milk"])
        .assert()
        .success();

    assert!(env.dir().join("a.json").exists());
    assert!(!env.notebook().exists());
}

#[test]
fn add_uses_the_default_location_without_notes_notebook() {
    let env = TestEnv::new();

    env.notes()
        .env_remove("NOTES_NOTEBOOK")
        .args(["add", "Buy milk"])
        .assert()
        .success();

    let expected = if cfg!(target_os = "macos") {
        env.dir()
            .join("Library")
            .join("Application Support")
            .join("notes")
            .join("notebook.json")
    } else {
        env.dir().join("notes").join("notebook.json")
    };
    assert_eq!(read_json(&expected)["notes"][0]["text"], json!("Buy milk"));
}
