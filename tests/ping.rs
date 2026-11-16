//! Binary tests of `notes ping`.

mod common;

use std::fs;

use common::TestEnv;
use predicates::str::contains;

/// The line `notes ping -n <times>` must print: `pong` `times` times, joined
/// by single spaces, then one newline.
fn pong_line(times: usize) -> String {
    format!("{}\n", vec!["pong"; times].join(" "))
}

#[test]
fn ping_without_option_prints_one_pong() {
    let env = TestEnv::new();

    env.notes()
        .arg("ping")
        .assert()
        .success()
        .code(0)
        .stdout("pong\n")
        .stderr("");
}

#[test]
fn ping_with_short_option_prints_pongs_on_one_line() {
    let env = TestEnv::new();

    env.notes()
        .args(["ping", "-n", "5"])
        .assert()
        .success()
        .code(0)
        .stdout("pong pong pong pong pong\n")
        .stderr("");
}

#[test]
fn ping_with_long_option_prints_pongs_on_one_line() {
    let env = TestEnv::new();

    env.notes()
        .args(["ping", "--times", "3"])
        .assert()
        .success()
        .code(0)
        .stdout("pong pong pong\n")
        .stderr("");
}

#[test]
fn ping_with_one_prints_one_pong() {
    let env = TestEnv::new();

    env.notes()
        .args(["ping", "-n", "1"])
        .assert()
        .success()
        .code(0)
        .stdout("pong\n")
        .stderr("");
}

#[test]
fn ping_with_the_maximum_prints_a_thousand_pongs() {
    let env = TestEnv::new();

    env.notes()
        .args(["ping", "-n", "1000"])
        .assert()
        .success()
        .code(0)
        .stdout(pong_line(1000))
        .stderr("");
}

#[test]
fn ping_refuses_values_outside_the_range_as_usage_errors() {
    let env = TestEnv::new();

    for value in ["0", "1001", "-1", "abc"] {
        env.notes()
            .args(["ping", "-n", value])
            .assert()
            .code(2)
            .stdout("")
            .stderr(predicates::str::starts_with("error:"));
    }
}

#[test]
fn ping_answers_when_the_notebook_is_invalid_and_leaves_it_unchanged() {
    let env = TestEnv::new();
    fs::write(env.notebook(), "not json").unwrap();

    env.notes()
        .arg("ping")
        .assert()
        .success()
        .code(0)
        .stdout("pong\n")
        .stderr("");

    assert_eq!(fs::read(env.notebook()).unwrap(), b"not json");
}

#[test]
fn ping_does_not_create_the_notebook_or_its_directory() {
    let env = TestEnv::new();
    let directory = env.dir().join("missing");
    let notebook = directory.join("notebook.json");

    env.notes()
        .env("NOTES_NOTEBOOK", &notebook)
        .arg("ping")
        .assert()
        .success()
        .code(0);

    assert!(!notebook.exists());
    assert!(!directory.exists());
}

#[test]
fn ping_ignores_the_notebook_flag() {
    let env = TestEnv::new();

    env.notes()
        .args(["--notebook", "other.json", "ping"])
        .assert()
        .success()
        .code(0)
        .stdout("pong\n");

    assert!(!env.dir().join("other.json").exists());
}

#[test]
fn help_lists_ping_with_its_description() {
    let env = TestEnv::new();

    env.notes()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"(?m)^\s+ping\s+Reply with pong$").unwrap());
}

#[test]
fn ping_help_describes_the_option_and_its_default() {
    let env = TestEnv::new();

    env.notes()
        .args(["ping", "--help"])
        .assert()
        .success()
        .stdout(contains("-n, --times <N>"))
        .stdout(contains("Number of pongs, 1-1000"))
        .stdout(contains("[default: 1]"));
}
