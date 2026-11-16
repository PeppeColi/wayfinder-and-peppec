//! Binary tests of `notes list`.

mod common;

use std::fs;
use std::io::Read;
use std::process::Stdio;

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
fn list_prints_notes_most_recent_first() {
    let env = TestEnv::new();
    three_notes(&env);

    let expected = [
        "2026-11-15 22:00  Buy milk  [home, shopping]",
        "2026-11-15 21:00  Call Anna",
        "2026-11-14 09:05  first",
        "                  second  [work]",
        "",
    ]
    .join("\n");
    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn list_uses_the_time_zone_from_tz() {
    let env = TestEnv::new();
    three_notes(&env);

    let out = env
        .notes()
        .env("TZ", "Europe/Rome")
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("2026-11-15 23:00"), "{stdout}");
}

#[test]
fn list_ties_print_the_later_note_in_the_file_first() {
    let env = TestEnv::new();
    write_notebook(
        &env,
        json!([
            { "text": "one", "tags": [], "created": "2026-11-15T21:00:00Z" },
            { "text": "two", "tags": [], "created": "2026-11-15T21:00:00Z" }
        ]),
    );

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout("2026-11-15 21:00  two\n2026-11-15 21:00  one\n");
}

#[test]
fn list_orders_a_later_note_with_an_earlier_instant_by_its_instant() {
    let env = TestEnv::new();
    write_notebook(
        &env,
        json!([
            { "text": "new", "tags": [], "created": "2026-11-15T21:00:00Z" },
            { "text": "old", "tags": [], "created": "2026-11-14T21:00:00Z" }
        ]),
    );

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout("2026-11-15 21:00  new\n2026-11-14 21:00  old\n");
}

#[test]
fn list_of_a_missing_notebook_says_no_notes_and_creates_nothing() {
    let env = TestEnv::new();

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout("")
        .stderr("No notes.\n");

    assert!(!env.notebook().exists());
}

#[test]
fn list_of_an_empty_notebook_says_no_notes() {
    let env = TestEnv::new();
    write_notebook(&env, json!([]));

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout("")
        .stderr("No notes.\n");
}

#[test]
fn list_stops_quietly_when_stdout_is_closed_early() {
    let env = TestEnv::new();
    let notes: Vec<_> = (0..20_000)
        .map(|i| {
            json!({
                "text": format!("note number {i}"),
                "tags": [],
                "created": "2026-11-15T21:00:00Z"
            })
        })
        .collect();
    write_notebook(&env, json!(notes));

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin!("notes"))
        .env_clear()
        .env("NOTES_NOTEBOOK", env.notebook())
        .env("TZ", "UTC")
        .env("HOME", env.dir())
        .arg("list")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut byte = [0u8; 1];
    stdout.read_exact(&mut byte).unwrap();
    drop(stdout);

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(output.stderr, b"");
}
