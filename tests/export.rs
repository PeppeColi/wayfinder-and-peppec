//! Binary tests of `notes export`.

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

fn file_names(env: &TestEnv) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(env.dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

const THREE_NOTES_MD: &str = "# Notes\n\n## 2026-11-15 22:00\n\nBuy milk\n\nTags: home, shopping\n\n## 2026-11-15 21:00\n\nCall Anna\n\n## 2026-11-14 09:05\n\nfirst\nsecond\n\nTags: work\n";

#[test]
fn export_writes_the_whole_notebook_as_markdown() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes()
        .args(["export", "notes.md"])
        .assert()
        .success()
        .stdout("")
        .stderr("Exported 3 notes to notes.md\n");

    assert_eq!(
        fs::read_to_string(env.dir().join("notes.md")).unwrap(),
        THREE_NOTES_MD
    );
}

#[test]
fn export_of_one_note_says_note_in_the_singular() {
    let env = TestEnv::new();
    write_notebook(
        &env,
        json!([{ "text": "a", "tags": [], "created": "2026-11-15T21:00:00Z" }]),
    );

    env.notes()
        .args(["export", "out.md"])
        .assert()
        .success()
        .stderr("Exported 1 note to out.md\n");
    assert_eq!(
        fs::read_to_string(env.dir().join("out.md")).unwrap(),
        "# Notes\n\n## 2026-11-15 21:00\n\na\n"
    );
}

#[test]
fn export_of_a_missing_notebook_writes_an_empty_export_and_keeps_it_missing() {
    let env = TestEnv::new();

    env.notes()
        .args(["export", "out.md"])
        .assert()
        .success()
        .stdout("")
        .stderr("Exported 0 notes to out.md\n");

    assert_eq!(
        fs::read_to_string(env.dir().join("out.md")).unwrap(),
        "# Notes\n"
    );
    assert!(!env.notebook().exists());
}

#[test]
fn export_refuses_an_existing_target_without_force() {
    let env = TestEnv::new();
    three_notes(&env);
    fs::write(env.dir().join("out.md"), "old").unwrap();

    env.notes()
        .args(["export", "out.md"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr("error: out.md already exists; use --force to overwrite\n");

    assert_eq!(fs::read_to_string(env.dir().join("out.md")).unwrap(), "old");
}

#[test]
fn export_with_force_replaces_the_target() {
    let env = TestEnv::new();
    three_notes(&env);
    fs::write(env.dir().join("out.md"), "old").unwrap();

    env.notes()
        .args(["export", "--force", "out.md"])
        .assert()
        .success()
        .stderr("Exported 3 notes to out.md\n");

    assert_eq!(
        fs::read_to_string(env.dir().join("out.md")).unwrap(),
        THREE_NOTES_MD
    );
}

#[test]
fn export_never_creates_a_missing_directory() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes()
        .args(["export", "missing/out.md"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr("error: directory missing does not exist\n");

    assert!(!env.dir().join("missing").exists());
}

#[cfg(unix)]
#[test]
fn export_to_a_read_only_directory_fails_with_the_reason() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();
    three_notes(&env);
    let ro = env.dir().join("ro");
    fs::create_dir(&ro).unwrap();
    fs::set_permissions(&ro, fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores directory permissions: nothing to check then.
    if fs::write(ro.join("probe"), "").is_ok() {
        return;
    }

    let output = env.notes().args(["export", "ro/out.md"]).output().unwrap();

    fs::set_permissions(&ro, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.starts_with("error: cannot write ro/out.md: permission denied"),
        "{stderr}"
    );
    assert_eq!(fs::read_dir(&ro).unwrap().count(), 0);
}

#[test]
fn two_exports_of_the_same_notebook_are_byte_identical() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes().args(["export", "a.md"]).assert().success();
    env.notes().args(["export", "b.md"]).assert().success();

    assert_eq!(
        fs::read(env.dir().join("a.md")).unwrap(),
        fs::read(env.dir().join("b.md")).unwrap()
    );
}

#[test]
fn export_leaves_no_temporary_file_behind() {
    let env = TestEnv::new();
    three_notes(&env);
    fs::write(env.dir().join("old.md"), "old").unwrap();

    env.notes().args(["export", "out.md"]).assert().success();
    env.notes().args(["export", "old.md"]).assert().failure();
    env.notes()
        .args(["export", "--force", "old.md"])
        .assert()
        .success();

    assert_eq!(file_names(&env), ["notebook.json", "old.md", "out.md"]);
}

#[test]
fn export_uses_the_time_zone_from_tz() {
    let env = TestEnv::new();
    three_notes(&env);

    env.notes()
        .env("TZ", "Europe/Rome")
        .args(["export", "out.md"])
        .assert()
        .success();

    let content = fs::read_to_string(env.dir().join("out.md")).unwrap();
    assert_eq!(content.lines().nth(2), Some("## 2026-11-15 23:00"));
}
