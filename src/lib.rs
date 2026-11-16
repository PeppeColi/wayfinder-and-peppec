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
use jiff::tz::TimeZone;
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
    /// The text of a Note is empty once trimmed.
    EmptyText,
    /// A tag breaks the Tag rules (tag as given).
    InvalidTag(String),
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
    /// The output cannot be written to stdout (any error but a closed pipe).
    WriteOutput { source: io::Error },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyText => write!(f, "note text is empty"),
            Error::InvalidTag(tag) => {
                write!(
                    f,
                    "invalid tag '{tag}': use 1-32 letters, digits, '-' or '_', starting with a letter or digit"
                )
            }
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
            Error::WriteOutput { source } => {
                write!(f, "cannot write output: {}", source.kind())
            }
        }
    }
}

impl std::error::Error for Error {}

/// Normalises the text of a Note: every `\r\n` and lone `\r` becomes `\n`,
/// then leading and trailing white space is trimmed. The inside is kept as
/// it is. Text that is empty after trimming is an error.
pub fn normalise_text(text: &str) -> Result<String, Error> {
    let lines = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = lines.trim();
    if trimmed.is_empty() {
        Err(Error::EmptyText)
    } else {
        Ok(trimmed.to_owned())
    }
}

/// Longest valid tag, in characters.
const MAX_TAG_LEN: usize = 32;

/// Checks the Tag rules and returns the tag in lowercase: 1 to 32 ASCII
/// letters, digits, `-` or `_`, starting with a letter or a digit.
pub fn validate_tag(tag: &str) -> Result<String, Error> {
    let valid = (1..=MAX_TAG_LEN).contains(&tag.len())
        && tag.starts_with(|c: char| c.is_ascii_alphanumeric())
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(tag.to_ascii_lowercase())
    } else {
        Err(Error::InvalidTag(tag.to_owned()))
    }
}

/// Validates and lowercases `tags` in the order given, dropping repeats and
/// keeping the order of first occurrence. The first invalid tag is the error.
pub fn normalise_tags<S: AsRef<str>>(tags: &[S]) -> Result<Vec<String>, Error> {
    let mut result: Vec<String> = Vec::new();
    for tag in tags {
        let tag = validate_tag(tag.as_ref())?;
        if !result.contains(&tag) {
            result.push(tag);
        }
    }
    Ok(result)
}

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

/// Width of `YYYY-MM-DD HH:MM  `: the indent of continuation lines.
const CONTINUATION_INDENT: usize = 18;

/// The notes most recent first: by `created` descending; at equal instants the
/// note later in `notes` comes first.
pub fn order_notes(notes: &[Note]) -> Vec<&Note> {
    let mut indexed: Vec<(usize, &Note)> = notes.iter().enumerate().collect();
    indexed.sort_by(|(ia, a), (ib, b)| b.created.cmp(&a.created).then(ib.cmp(ia)));
    indexed.into_iter().map(|(_, note)| note).collect()
}

/// One note as listed: local `YYYY-MM-DD HH:MM`, two spaces, the text
/// (continuation lines indented), then `  [tag1, tag2]` if it has tags. No
/// trailing newline.
pub fn format_note(note: &Note, tz: &TimeZone) -> String {
    let when = tz.to_datetime(note.created).strftime("%Y-%m-%d %H:%M");
    let indent = " ".repeat(CONTINUATION_INDENT);
    let mut line = format!("{when}  ");
    for (i, part) in note.text.split('\n').enumerate() {
        if i > 0 {
            line.push('\n');
            line.push_str(&indent);
        }
        line.push_str(part);
    }
    if !note.tags.is_empty() {
        line.push_str("  [");
        line.push_str(&note.tags.join(", "));
        line.push(']');
    }
    line
}

/// Writes the notes, most recent first, one per line. A closed pipe stops the
/// output quietly and succeeds; any other write error is an [`Error`].
pub fn write_notes<W: Write>(out: &mut W, notes: &[Note], tz: &TimeZone) -> Result<(), Error> {
    for note in order_notes(notes) {
        match writeln!(out, "{}", format_note(note, tz)) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(source) => return Err(Error::WriteOutput { source }),
        }
    }
    match out.flush() {
        Err(source) if source.kind() != io::ErrorKind::BrokenPipe => {
            Err(Error::WriteOutput { source })
        }
        _ => Ok(()),
    }
}

/// `notes list`: prints every note of the Notebook at `path` to `out`; when
/// there are none, prints `No notes.` to `err` instead. Never creates the file.
pub fn list<W: Write, E: Write>(
    path: &Path,
    tz: &TimeZone,
    out: &mut W,
    err: &mut E,
) -> Result<(), Error> {
    let notebook = read_notebook(path)?;
    if notebook.notes.is_empty() {
        let _ = writeln!(err, "No notes.");
        return Ok(());
    }
    write_notes(out, &notebook.notes, tz)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(text: &str, tags: &[&str], created: &str) -> Note {
        Note {
            text: text.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            created: created.parse().unwrap(),
        }
    }

    fn texts(notes: &[Note]) -> Vec<&str> {
        order_notes(notes).iter().map(|n| n.text.as_str()).collect()
    }

    #[test]
    fn order_is_created_descending() {
        let notes = [
            note("a", &[], "2026-11-14T09:00:00Z"),
            note("b", &[], "2026-11-15T09:00:00Z"),
        ];
        assert_eq!(texts(&notes), ["b", "a"]);
    }

    #[test]
    fn order_ties_put_the_later_note_first() {
        let notes = [
            note("a", &[], "2026-11-14T09:00:00Z"),
            note("b", &[], "2026-11-14T09:00:00Z"),
            note("c", &[], "2026-11-14T09:00:00Z"),
        ];
        assert_eq!(texts(&notes), ["c", "b", "a"]);
    }

    #[test]
    fn order_uses_the_instant_even_when_a_later_note_has_an_earlier_one() {
        let notes = [
            note("a", &[], "2026-11-15T09:00:00Z"),
            note("b", &[], "2026-11-14T09:00:00Z"),
        ];
        assert_eq!(texts(&notes), ["a", "b"]);
    }

    #[test]
    fn format_with_tags() {
        let n = note("Buy milk", &["home", "shopping"], "2026-11-15T22:00:00Z");
        assert_eq!(
            format_note(&n, &TimeZone::UTC),
            "2026-11-15 22:00  Buy milk  [home, shopping]"
        );
    }

    #[test]
    fn format_without_tags_has_no_trailing_spaces() {
        let n = note("Call Anna", &[], "2026-11-15T21:00:00Z");
        assert_eq!(
            format_note(&n, &TimeZone::UTC),
            "2026-11-15 21:00  Call Anna"
        );
    }

    #[test]
    fn format_multi_line_indents_continuations_and_tags_follow_the_last_line() {
        let n = note("first\nsecond", &["work"], "2026-11-14T09:05:00Z");
        assert_eq!(
            format_note(&n, &TimeZone::UTC),
            "2026-11-14 09:05  first\n                  second  [work]"
        );
    }

    #[test]
    fn format_uses_the_time_zone_passed_in() {
        let n = note("x", &[], "2026-11-15T22:00:00Z");
        let rome = TimeZone::get("Europe/Rome").unwrap();
        assert_eq!(format_note(&n, &rome), "2026-11-15 23:00  x");
    }

    struct FailingWriter(io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn write_notes_broken_pipe_is_a_quiet_success() {
        let notes = [note("a", &[], "2026-11-14T09:00:00Z")];
        let mut out = FailingWriter(io::ErrorKind::BrokenPipe);
        assert!(write_notes(&mut out, &notes, &TimeZone::UTC).is_ok());
    }

    #[test]
    fn write_notes_other_errors_fail_with_the_reason() {
        let notes = [note("a", &[], "2026-11-14T09:00:00Z")];
        let mut out = FailingWriter(io::ErrorKind::PermissionDenied);
        let err = write_notes(&mut out, &notes, &TimeZone::UTC).unwrap_err();
        assert_eq!(err.to_string(), "cannot write output: permission denied");
    }

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
    fn validate_tag_accepts_the_boundaries() {
        assert_eq!(validate_tag("a").unwrap(), "a");
        assert_eq!(validate_tag("9").unwrap(), "9");
        assert_eq!(validate_tag("a-b_9").unwrap(), "a-b_9");
        assert_eq!(validate_tag("x-").unwrap(), "x-");
        assert_eq!(validate_tag(&"A".repeat(32)).unwrap(), "a".repeat(32));
    }

    #[test]
    fn validate_tag_lowercases() {
        assert_eq!(validate_tag("HoMe").unwrap(), "home");
    }

    #[test]
    fn validate_tag_rejects_invalid_tags_showing_them_as_typed() {
        for tag in ["", "_x", "-x", "a b", "caffè", "b@d", "c!"] {
            assert_eq!(
                validate_tag(tag).unwrap_err().to_string(),
                format!(
                    "invalid tag '{tag}': use 1-32 letters, digits, '-' or '_', starting with a letter or digit"
                )
            );
        }
        assert!(validate_tag(&"a".repeat(33)).is_err());
    }

    #[test]
    fn normalise_tags_deduplicates_keeping_first_occurrence_order() {
        assert_eq!(normalise_tags(&["b", "a", "B"]).unwrap(), ["b", "a"]);
        assert_eq!(
            normalise_tags(&["Home", "shopping", "HOME"]).unwrap(),
            ["home", "shopping"]
        );
    }

    #[test]
    fn normalise_tags_accepts_no_tags() {
        assert!(normalise_tags::<&str>(&[]).unwrap().is_empty());
    }

    #[test]
    fn normalise_tags_reports_the_first_invalid_tag() {
        let err = normalise_tags(&["ok", "b@d", "c!"]).unwrap_err();
        assert!(matches!(err, Error::InvalidTag(tag) if tag == "b@d"));
    }

    #[test]
    fn normalise_text_converts_every_line_ending_to_newline() {
        assert_eq!(normalise_text("a\r\nb\rc\nd").unwrap(), "a\nb\nc\nd");
        assert_eq!(normalise_text("a\r\r\nb").unwrap(), "a\n\nb");
    }

    #[test]
    fn normalise_text_trims_unicode_white_space_at_both_ends() {
        assert_eq!(normalise_text("  Buy milk \r\n").unwrap(), "Buy milk");
        assert_eq!(normalise_text("\u{3000}caffè\u{2003}").unwrap(), "caffè");
    }

    #[test]
    fn normalise_text_keeps_the_inside_unchanged() {
        assert_eq!(
            normalise_text("  para one\n\n    indented\tcol  ").unwrap(),
            "para one\n\n    indented\tcol"
        );
    }

    #[test]
    fn normalise_text_refuses_empty_or_blank_text() {
        for text in ["", " \t\r\n ", "\r", "\u{00a0}"] {
            let err = normalise_text(text).unwrap_err();
            assert_eq!(err.to_string(), "note text is empty");
        }
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
