//! Shared helper for the binary tests: every `notes` command is built here.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// The instant every command sees unless a test overrides `NOTES_NOW`.
pub const NOW: &str = "2026-11-15T21:00:00Z";

/// A fresh temporary home for one test, with the Notebook inside it.
pub struct TestEnv {
    dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("create temporary directory"),
        }
    }

    /// The temporary directory used as `HOME` and `XDG_DATA_HOME`.
    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    /// The Notebook file that `NOTES_NOTEBOOK` points to.
    pub fn notebook(&self) -> PathBuf {
        self.dir().join("notebook.json")
    }

    /// A `notes` command with a cleared environment: `HOME` and `XDG_DATA_HOME`
    /// in the temporary directory, `NOTES_NOTEBOOK` a file in it, `TZ=UTC` and
    /// `NOTES_NOW` set to [`NOW`]. Tests override or remove single variables.
    pub fn notes(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("notes"));
        cmd.env_clear()
            .current_dir(self.dir())
            .env("HOME", self.dir())
            .env("XDG_DATA_HOME", self.dir())
            .env("NOTES_NOTEBOOK", self.notebook())
            .env("TZ", "UTC")
            .env("NOTES_NOW", NOW);
        cmd
    }
}

/// Parses a Notebook file as JSON.
pub fn read_json(path: &Path) -> Value {
    let content = fs::read_to_string(path).expect("read Notebook");
    serde_json::from_str(&content).expect("parse Notebook")
}
