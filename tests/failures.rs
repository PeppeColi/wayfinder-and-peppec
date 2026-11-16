//! Binary tests of how Notebook failures are reported.

mod common;

use std::fs;

use common::TestEnv;

fn corrupt(env: &TestEnv) {
    fs::write(env.notebook(), "not json").unwrap();
}

/// The text `serde_json` gives for `content` as a Notebook.
fn parser_message(content: &str) -> String {
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Shape {
        version: u64,
        notes: Vec<serde_json::Value>,
    }
    serde_json::from_str::<Shape>(content)
        .err()
        .unwrap()
        .to_string()
}

fn run(env: &TestEnv, args: &[&str], notes_now: Option<&str>) -> (i32, String, String) {
    let mut cmd = env.notes();
    cmd.args(args);
    if let Some(now) = notes_now {
        cmd.env("NOTES_NOW", now);
    }
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn unsupported_version_is_reported_with_the_path() {
    let env = TestEnv::new();
    fs::write(env.notebook(), r#"{"version": 2, "notes": []}"#).unwrap();
    let (code, out, err) = run(&env, &["list"], None);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert_eq!(
        err,
        format!(
            "error: unsupported Notebook version 2 in {}\n",
            env.notebook().display()
        )
    );
}

#[test]
fn invalid_json_and_missing_version_are_reported_with_the_parser_message() {
    let env = TestEnv::new();
    let path = env.notebook();
    for (content, message) in [
        ("not json", parser_message("not json")),
        (r#"{"notes": []}"#, parser_message(r#"{"notes": []}"#)),
    ] {
        fs::write(&path, content).unwrap();
        let (code, out, err) = run(&env, &["list"], None);
        assert_eq!(code, 1);
        assert!(out.is_empty());
        assert_eq!(
            err,
            format!("error: invalid Notebook {}: {message}\n", path.display())
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
}

#[test]
fn every_command_reports_the_same_read_failure() {
    let env = TestEnv::new();
    corrupt(&env);
    let expected = format!(
        "error: invalid Notebook {}: {}\n",
        env.notebook().display(),
        parser_message("not json")
    );
    for args in [
        vec!["add", "x"],
        vec!["list"],
        vec!["search", "tag"],
        vec!["export", "out.md"],
    ] {
        let (code, out, err) = run(&env, &args, None);
        assert_eq!(code, 1, "{args:?}");
        assert!(out.is_empty());
        assert_eq!(err, expected, "{args:?}");
    }
    assert_eq!(fs::read_to_string(env.notebook()).unwrap(), "not json");
    assert!(!env.dir().join("out.md").exists());
}

#[cfg(unix)]
#[test]
fn unreadable_notebook_is_reported_with_the_reason() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();
    fs::write(env.notebook(), r#"{"version":1,"notes":[]}"#).unwrap();
    fs::set_permissions(env.notebook(), fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(env.notebook()).is_ok() {
        return; // root ignores permissions
    }
    let (code, out, err) = run(&env, &["list"], None);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert_eq!(
        err,
        format!(
            "error: cannot read Notebook {}: permission denied\n",
            env.notebook().display()
        )
    );
}

#[test]
fn a_directory_as_notebook_cannot_be_read() {
    let env = TestEnv::new();
    let dir = env.dir().join("adir");
    fs::create_dir(&dir).unwrap();
    let (code, out, err) = run(&env, &["list", "--notebook", dir.to_str().unwrap()], None);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(
        err.starts_with(&format!("error: cannot read Notebook {}: ", dir.display())),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1);
}

#[test]
fn a_file_in_place_of_the_parent_directory_cannot_be_created() {
    let env = TestEnv::new();
    let blocker = env.dir().join("blocker");
    fs::write(&blocker, "").unwrap();
    let path = blocker.join("sub").join("notebook.json");
    let (code, out, err) = run(
        &env,
        &["add", "x", "--notebook", path.to_str().unwrap()],
        None,
    );
    assert_eq!(code, 1);
    assert!(out.is_empty());
    let dir = path.parent().unwrap();
    assert!(
        err.starts_with(&format!(
            "error: cannot create directory {}: ",
            dir.display()
        )),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1);
}

#[cfg(unix)]
#[test]
fn a_read_only_directory_cannot_be_written() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();
    let ro = env.dir().join("ro");
    fs::create_dir(&ro).unwrap();
    let path = ro.join("notebook.json");
    let content = r#"{"version":1,"notes":[]}"#;
    fs::write(&path, content).unwrap();
    fs::set_permissions(&ro, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(ro.join("probe"), "").is_ok() {
        return; // root ignores permissions
    }
    let (code, out, err) = run(
        &env,
        &["add", "x", "--notebook", path.to_str().unwrap()],
        None,
    );
    let left = fs::read_dir(&ro).unwrap().count();
    fs::set_permissions(&ro, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(
        err.starts_with(&format!(
            "error: cannot write Notebook {}: ",
            path.display()
        )),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    assert_eq!(left, 1);
}

#[test]
fn inputs_are_checked_in_the_fixed_order_before_the_notebook() {
    let env = TestEnv::new();
    corrupt(&env);
    let (code, out, err) = run(&env, &["add", "", "-t", "caffè"], Some("xyz"));
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(err, "error: note text is empty\n");

    let (_, _, err) = run(&env, &["add", "x", "-t", "caffè"], None);
    assert_eq!(
        err,
        "error: invalid tag 'caffè': use 1-32 letters, digits, '-' or '_', starting with a letter or digit\n"
    );

    let (_, _, err) = run(&env, &["add", "x"], Some("xyz"));
    assert_eq!(
        err,
        "error: invalid NOTES_NOW 'xyz': expected an RFC 3339 instant\n"
    );
}

#[test]
fn list_ignores_notes_now() {
    let env = TestEnv::new();
    let (code, _, _) = run(&env, &["list"], Some("xyz"));
    assert_eq!(code, 0);
}
