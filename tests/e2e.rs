//! End-to-end tests: journeys across several `notes` commands against one Notebook.
//!
//! This target is declared with `test = false`, so plain `cargo test` skips it.
//! Run the journeys of specification 12 with `cargo test --test e2e spec_12_`.

mod common;

use std::fs;

use common::TestEnv;

const INVALID_TAG_HINT: &str =
    "use 1-32 letters, digits, '-' or '_', starting with a letter or digit";

/// Adds one note at the instant `now`, expecting a silent success.
fn add(env: &TestEnv, now: &str, args: &[&str]) {
    env.notes()
        .env("NOTES_NOW", now)
        .arg("add")
        .args(args)
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

/// Adds five notes at different instants: one multi-line, one without tags,
/// two at the same instant, tags in mixed case and repeated.
fn add_journey_notes(env: &TestEnv) {
    add(
        env,
        "2026-11-15T09:00:00Z",
        &["Buy milk", "-t", "Home", "-t", "shopping", "-t", "HOME"],
    );
    add(
        env,
        "2026-11-14T08:30:00Z",
        &["  first line\nsecond line\n\n", "-t", "Work"],
    );
    add(env, "2026-11-15T12:00:00Z", &["Call Anna"]);
    add(env, "2026-11-16T07:00:00Z", &["Pay rent", "-t", "home"]);
    add(env, "2026-11-16T07:00:00Z", &["Water plants", "-t", "home"]);
}

/// What `list` prints after [`add_journey_notes`].
const JOURNEY_LIST: &str = concat!(
    "2026-11-16 07:00  Water plants  [home]\n",
    "2026-11-16 07:00  Pay rent  [home]\n",
    "2026-11-15 12:00  Call Anna\n",
    "2026-11-15 09:00  Buy milk  [home, shopping]\n",
    "2026-11-14 08:30  first line\n",
    "                  second line  [work]\n",
);

/// What `export` writes after [`add_journey_notes`].
const JOURNEY_MD: &str = concat!(
    "# Notes\n",
    "\n",
    "## 2026-11-16 07:00\n",
    "\n",
    "Water plants\n",
    "\n",
    "Tags: home\n",
    "\n",
    "## 2026-11-16 07:00\n",
    "\n",
    "Pay rent\n",
    "\n",
    "Tags: home\n",
    "\n",
    "## 2026-11-15 12:00\n",
    "\n",
    "Call Anna\n",
    "\n",
    "## 2026-11-15 09:00\n",
    "\n",
    "Buy milk\n",
    "\n",
    "Tags: home, shopping\n",
    "\n",
    "## 2026-11-14 08:30\n",
    "\n",
    "first line\n",
    "second line\n",
    "\n",
    "Tags: work\n",
);

#[test]
fn spec_12_add_list_search() {
    let env = TestEnv::new();
    add_journey_notes(&env);

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout(JOURNEY_LIST)
        .stderr("");

    env.notes()
        .args(["search", "hOmE"])
        .assert()
        .success()
        .stdout(concat!(
            "2026-11-16 07:00  Water plants  [home]\n",
            "2026-11-16 07:00  Pay rent  [home]\n",
            "2026-11-15 09:00  Buy milk  [home, shopping]\n",
        ))
        .stderr("");
}

#[test]
fn spec_12_add_then_export() {
    let env = TestEnv::new();
    add_journey_notes(&env);
    let target = env.dir().join("notes.md");

    env.notes()
        .args(["export", "notes.md"])
        .assert()
        .success()
        .stdout("")
        .stderr("Exported 5 notes to notes.md\n");
    assert_eq!(fs::read_to_string(&target).unwrap(), JOURNEY_MD);

    fs::write(&target, "stale").unwrap();
    env.notes()
        .args(["export", "notes.md"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("error: notes.md already exists; use --force to overwrite\n");
    assert_eq!(fs::read_to_string(&target).unwrap(), "stale");

    env.notes()
        .args(["export", "--force", "notes.md"])
        .assert()
        .success()
        .stdout("")
        .stderr("Exported 5 notes to notes.md\n");
    assert_eq!(fs::read_to_string(&target).unwrap(), JOURNEY_MD);
}

// On Windows the data directory does not follow HOME or XDG_DATA_HOME, so
// the test could not stay inside its temporary directory.
#[test]
#[cfg(not(windows))]
fn spec_12_first_use_default_location() {
    let env = TestEnv::new();
    let default_notebook = if cfg!(target_os = "macos") {
        env.dir()
            .join("Library")
            .join("Application Support")
            .join("notes")
            .join("notebook.json")
    } else {
        env.dir().join("notes").join("notebook.json")
    };

    env.notes()
        .env_remove("NOTES_NOTEBOOK")
        .arg("list")
        .assert()
        .success()
        .stdout("")
        .stderr("No notes.\n");
    assert_eq!(fs::read_dir(env.dir()).unwrap().count(), 0);

    env.notes()
        .env_remove("NOTES_NOTEBOOK")
        .args(["add", "Buy milk", "-t", "home"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
    assert!(default_notebook.is_file());
    assert!(!env.notebook().exists());

    env.notes()
        .env_remove("NOTES_NOTEBOOK")
        .arg("list")
        .assert()
        .success()
        .stdout("2026-11-15 21:00  Buy milk  [home]\n")
        .stderr("");
}

#[test]
fn spec_12_failed_add_changes_nothing() {
    let env = TestEnv::new();
    add(&env, "2026-11-15T09:00:00Z", &["Buy milk", "-t", "home"]);
    let before = fs::read(env.notebook()).unwrap();

    env.notes()
        .env("NOTES_NOW", "2026-11-15T10:00:00Z")
        .args(["add", "Call Anna", "-t", "work", "-t", "b@d"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(format!("error: invalid tag 'b@d': {INVALID_TAG_HINT}\n"));
    assert_eq!(fs::read(env.notebook()).unwrap(), before);

    env.notes()
        .arg("list")
        .assert()
        .success()
        .stdout("2026-11-15 09:00  Buy milk  [home]\n")
        .stderr("");

    env.notes()
        .args(["export", "notes.md"])
        .assert()
        .success()
        .stdout("")
        .stderr("Exported 1 note to notes.md\n");
    assert_eq!(
        fs::read_to_string(env.dir().join("notes.md")).unwrap(),
        "# Notes\n\n## 2026-11-15 09:00\n\nBuy milk\n\nTags: home\n"
    );
}
