//! Take notes from the terminal, tag them, and find them again.
//!
//! The library holds all the logic of the `notes` binary. Its functions
//! receive the current instant as a parameter: they never read the clock or
//! the environment.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// The only Notebook format version this program reads and writes.
pub const NOTEBOOK_VERSION: u64 = 1;

/// A piece of text saved by the user, with the instant it was created and
/// zero or more tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub text: String,
    pub tags: Vec<String>,
    pub created: Timestamp,
}

/// All the notes of the user, in insertion order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notebook {
    pub notes: Vec<Note>,
}

/// The Notebook as stored on disk.
#[derive(Serialize)]
struct NotebookFileOut<'a> {
    version: u64,
    notes: &'a [Note],
}

#[derive(Deserialize)]
struct NotebookFileIn {
    version: u64,
    notes: Vec<Note>,
}

/// A failure of a `notes` command; its `Display` is the message printed after
/// `error: `.
#[derive(Debug)]
pub enum Error {
    /// `NOTES_NOW` does not hold an RFC 3339 instant (value as given).
    InvalidNow(String),
    /// No data directory and no `--notebook`/`NOTES_NOTEBOOK`.
    NoDataDirectory,
    /// The Notebook exists but cannot be read.
    ReadNotebook { path: PathBuf, source: io::Error },
    /// The Notebook is not valid JSON or has the wrong shape.
    InvalidNotebook { path: PathBuf, message: String },
    /// The Notebook has a version other than [`NOTEBOOK_VERSION`].
    UnsupportedVersion { path: PathBuf, version: u64 },
    /// A missing parent directory of the Notebook cannot be created.
    CreateDirectory { path: PathBuf, source: io::Error },
    /// The Notebook cannot be written.
    WriteNotebook { path: PathBuf, source: io::Error },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidNow(value) => {
                write!(
                    f,
                    "invalid NOTES_NOW '{value}': expected an RFC 3339 instant"
                )
            }
            Error::NoDataDirectory => {
                write!(
                    f,
                    "cannot find a data directory; use --notebook or NOTES_NOTEBOOK"
                )
            }
            Error::ReadNotebook { path, source } => {
                write!(
                    f,
                    "cannot read Notebook {}: {}",
                    path.display(),
                    source.kind()
                )
            }
            Error::InvalidNotebook { path, message } => {
                write!(f, "invalid Notebook {}: {message}", path.display())
            }
            Error::UnsupportedVersion { path, version } => {
                write!(
                    f,
                    "unsupported Notebook version {version} in {}",
                    path.display()
                )
            }
            Error::CreateDirectory { path, source } => {
                write!(
                    f,
                    "cannot create directory {}: {}",
                    path.display(),
                    source.kind()
                )
            }
            Error::WriteNotebook { path, source } => {
                write!(
                    f,
                    "cannot write Notebook {}: {}",
                    path.display(),
                    source.kind()
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// Parses the value of `NOTES_NOW`: one RFC 3339 instant with offset.
pub fn parse_now(value: &str) -> Result<Timestamp, Error> {
    value
        .parse::<Timestamp>()
        .map_err(|_| Error::InvalidNow(value.to_owned()))
}

/// Resolves where the Notebook lives.
///
/// `notebook` is the `--notebook`/`NOTES_NOTEBOOK` value, if any (the flag
/// already chosen over the variable); it is always a file path and wins over
/// the default `<data_dir>/notes/notebook.json`.
pub fn notebook_location(
    notebook: Option<&Path>,
    data_dir: Option<&Path>,
) -> Result<PathBuf, Error> {
    match (notebook, data_dir) {
        (Some(notebook), _) => Ok(notebook.to_path_buf()),
        (None, Some(data_dir)) => Ok(data_dir.join("notes").join("notebook.json")),
        (None, None) => Err(Error::NoDataDirectory),
    }
}

/// Reads the Notebook at `path`; a missing file is an empty Notebook.
pub fn read_notebook(path: &Path) -> Result<Notebook, Error> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Notebook::default()),
        Err(source) => {
            return Err(Error::ReadNotebook {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    parse_notebook(&content, path)
}

fn parse_notebook(content: &str, path: &Path) -> Result<Notebook, Error> {
    let invalid = |err: serde_json::Error| Error::InvalidNotebook {
        path: path.to_path_buf(),
        message: err.to_string(),
    };
    let value: serde_json::Value = serde_json::from_str(content).map_err(invalid)?;
    if let Some(version) = value.get("version").and_then(serde_json::Value::as_u64)
        && version != NOTEBOOK_VERSION
    {
        return Err(Error::UnsupportedVersion {
            path: path.to_path_buf(),
            version,
        });
    }
    let file: NotebookFileIn = serde_json::from_value(value).map_err(invalid)?;
    debug_assert_eq!(file.version, NOTEBOOK_VERSION);
    Ok(Notebook { notes: file.notes })
}

/// Renders the Notebook as pretty-printed JSON, ending with a newline.
pub fn render_notebook(notebook: &Notebook) -> String {
    let file = NotebookFileOut {
        version: NOTEBOOK_VERSION,
        notes: &notebook.notes,
    };
    let mut json = serde_json::to_string_pretty(&file).expect("a Notebook always serialises");
    json.push('\n');
    json
}

/// Writes the Notebook atomically: a temporary file in the same directory,
/// then a rename over `path`. Missing parent directories are created.
pub fn write_notebook(path: &Path, notebook: &Notebook) -> Result<(), Error> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir).map_err(|source| Error::CreateDirectory {
        path: dir.to_path_buf(),
        source,
    })?;

    let write_error = |source| Error::WriteNotebook {
        path: path.to_path_buf(),
        source,
    };
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = dir.join(format!(".{file_name}.{}.tmp", process::id()));
    let result = write_file(&temp, render_notebook(notebook).as_bytes()).and_then(|()| {
        // Keep any permissions the user set on the existing Notebook.
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temp, metadata.permissions())?;
        }
        fs::rename(&temp, path)
    });
    if let Err(source) = result {
        let _ = fs::remove_file(&temp);
        return Err(write_error(source));
    }
    Ok(())
}

/// Writes `content` to a new file at `path`; fails if anything, even a
/// symlink, already exists there.
fn write_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(content)?;
    file.sync_all()
}

/// Adds a Note with `text` and `tags`, created at `now`, to the end of the
/// Notebook at `path`, creating the Notebook if it is missing.
pub fn add(path: &Path, text: &str, tags: &[String], now: Timestamp) -> Result<(), Error> {
    let mut notebook = read_notebook(path)?;
    notebook.notes.push(Note {
        text: text.to_owned(),
        tags: tags.to_vec(),
        created: now,
    });
    write_notebook(path, &notebook)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> Timestamp {
        value.parse().unwrap()
    }

    #[test]
    fn location_notebook_value_wins_over_the_default() {
        assert_eq!(
            notebook_location(Some(Path::new("a.json")), Some(Path::new("/data"))).unwrap(),
            PathBuf::from("a.json")
        );
    }

    #[test]
    fn location_notebook_value_is_used_without_a_data_directory() {
        assert_eq!(
            notebook_location(Some(Path::new("/x/b.json")), None).unwrap(),
            PathBuf::from("/x/b.json")
        );
    }

    #[test]
    fn location_defaults_to_the_data_directory() {
        assert_eq!(
            notebook_location(None, Some(Path::new("/data"))).unwrap(),
            Path::new("/data").join("notes").join("notebook.json")
        );
    }

    #[test]
    fn location_without_any_directory_is_an_error() {
        let err = notebook_location(None, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "cannot find a data directory; use --notebook or NOTES_NOTEBOOK"
        );
    }

    #[test]
    fn parse_now_accepts_rfc_3339_with_offset() {
        assert_eq!(
            parse_now("2026-11-15T22:00:00+01:00").unwrap(),
            instant("2026-11-15T21:00:00Z")
        );
    }

    #[test]
    fn parse_now_rejects_other_values() {
        for value in ["xyz", "", "2026-11-15T21:00:00", "2026-11-15"] {
            let err = parse_now(value).unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("invalid NOTES_NOW '{value}': expected an RFC 3339 instant")
            );
        }
    }

    #[test]
    fn notebook_round_trips_with_created_in_utc() {
        let notebook = Notebook {
            notes: vec![Note {
                text: "Buy milk".into(),
                tags: vec![],
                created: instant("2026-11-15T22:00:00+01:00"),
            }],
        };
        let json = render_notebook(&notebook);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "version": 1,
                "notes": [
                    { "text": "Buy milk", "tags": [], "created": "2026-11-15T21:00:00Z" }
                ]
            })
        );
        assert_eq!(
            parse_notebook(&json, Path::new("n.json")).unwrap(),
            notebook
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_notebook_does_not_follow_a_symlink_at_the_temporary_path() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "keep me").unwrap();
        let temp = dir
            .path()
            .join(format!(".notebook.json.{}.tmp", process::id()));
        std::os::unix::fs::symlink(&target, &temp).unwrap();

        let result = write_notebook(&dir.path().join("notebook.json"), &Notebook::default());

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "keep me");
        assert!(fs::symlink_metadata(&temp).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn write_notebook_keeps_the_permissions_of_the_existing_notebook() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notebook.json");
        write_notebook(&path, &Notebook::default()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

        write_notebook(&path, &Notebook::default()).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
