# Failures: design

Every failure of `notes` prints exactly one line on stderr, `error: <message>`, leaves stdout empty, and exits 1. clap's usage errors keep clap's own output and exit 2. When several things are wrong, only the first failure is reported, in a fixed order, and inputs are checked before any file is read or written. File errors name the path and a platform-independent reason. A closed stdout pipe is not a failure: the program stops quietly with exit 0.

This document resolves the wayfinder ticket "Which failures exist, and how is each one reported?" of the map "Map: Notes CLI v1".

```
$ notes add "x" -t caffè
error: invalid tag 'caffè': use 1-32 letters, digits, '-' or '_', starting with a letter or digit
$ echo $?
1
$ notes export notes.md
error: notes.md already exists; use --force to overwrite
$ notes list | head -n 1          # exit 0, nothing on stderr
2026-11-14 09:05  Buy milk  [home]
$ notes add "-5"
error: unexpected argument '-5' found        (clap, followed by its usage lines; exit 2)
```

## Terms

- **Notebook**: all the notes of the user, taken together. There is exactly one. _Avoid_: store, database, collection.
- **Export**: a copy of the notes written to a file the user chooses. _Avoid_: dump, backup.

## Why

Earlier decisions (tag rules, Notebook format, export file, note text, testing) each said "an error" and left the message and the exit code to this ticket. Binary tests compare stderr and the exit code exactly, so the specification needs one contract for every failure. PEPPEC's agent cannot run the program: everything below must be checkable by a test.

## Locked decisions

None. Every choice below can be revisited; the costliest change would be splitting exit code 1 into several codes once scripts depend on it.

## Routine choices

- **Exit codes**: 0 success; 1 every failure that is not a usage error; 2 clap usage errors (unknown option, missing `<TEXT>`, text starting with `-` without `--`, …). Rejected: one code per class (1 input, 3 Notebook, 4 file system); `sysexits.h` codes. Both add a mapping every work unit must get right, for one user who reads the message anyway.
- **Message shape**: one line on stderr, `error: <message>`, matching the prefix of clap's usage errors; stdout empty on every failure. A hint, when useful, sits inside the same line (`…; use --force to overwrite`). Rejected: `notes: <message>` (two styles next to clap's); a separate `hint:` line (doubles what every test pins).
- **Closed stdout pipe** (`notes list | head -n 1`): writing stops at the first `BrokenPipe` error, nothing is printed on stderr, exit 0. No `println!` (it panics on a closed pipe): output goes through a writer whose errors are handled. Any other stdout write error is a failure: `error: cannot write output: <reason>`, exit 1. Rejected: restoring the default SIGPIPE (needs `unsafe` libc code, platform-specific); exit 1 (breaks `set -o pipefail` scripts for a normal pipe). Testable as a unit test with a writer that fails with `BrokenPipe`, and as a binary test that spawns `notes list` on a large Notebook and closes the read end early.
- **Several failures at once**: only the first is reported, in this order: clap usage → note text (empty) → tags, in the order given → `NOTES_NOW` → Notebook location (data directory) → reading the Notebook (unreadable, invalid, version) → creating parent directories → writing (Notebook or export target: exists, directory missing, not writable). No file is read or written before the inputs are valid: an invalid tag or an empty text never touches the Notebook. Commands skip steps they do not need (`list` never checks `NOTES_NOW`; `list`/`search`/`export` never create directories or write the Notebook). Rejected: reporting every input error at once (breaks the one-line shape, multiplies test combinations).
- **Reason in file errors**: the path, then `: `, then the text of Rust's `io::ErrorKind` (lowercase and identical on macOS and Linux: `permission denied`, `entity not found`, `is a directory`, …), so the whole line is exact in tests on every platform. Invalid JSON is not an I/O error: its reason is `serde_json`'s message (`expected value at line 1 column 1`), also platform-independent. Rejected: the full OS text (`Permission denied (os error 13)`, differs by platform); the path only (hides why).
- **Paths in messages**: the export path exactly as the user typed it (as in the success line); its directory as `Path::parent` of that path. The Notebook path as resolved: the value of `--notebook` or `NOTES_NOTEBOOK` as given, or the default `<data_dir>/notes/notebook.json`. Paths are not quoted; values the user typed (tag, `NOTES_NOW`) are in single quotes.
- **Message catalogue** (each line printed as `error: <message>`, exit 1):

  | Failure | Message |
  |---|---|
  | Invalid tag (`add`, `search`) | `invalid tag '<TAG>': use 1-32 letters, digits, '-' or '_', starting with a letter or digit` (the tag as typed) |
  | Empty note text (`add`) | `note text is empty` |
  | Invalid `NOTES_NOW` | `invalid NOTES_NOW '<VALUE>': expected an RFC 3339 instant` |
  | No data directory, no `--notebook`/`NOTES_NOTEBOOK` | `cannot find a data directory; use --notebook or NOTES_NOTEBOOK` |
  | Notebook cannot be read (permissions, is a directory, …) | `cannot read Notebook <PATH>: <reason>` |
  | Notebook is not valid JSON or has the wrong shape (missing `version` included) | `invalid Notebook <PATH>: <parser message>` |
  | Notebook `version` other than 1 | `unsupported Notebook version <N> in <PATH>` |
  | Parent directory cannot be created (`add`) | `cannot create directory <DIR>: <reason>` |
  | Notebook cannot be written (`add`) | `cannot write Notebook <PATH>: <reason>` |
  | Export target exists, no `--force` | `<PATH> already exists; use --force to overwrite` |
  | Export directory missing | `directory <DIR> does not exist` |
  | Export target cannot be written | `cannot write <PATH>: <reason>` |
  | stdout write error other than a closed pipe | `cannot write output: <reason>` |

  A missing Notebook file is not a failure (it is an empty Notebook, decided with the Notebook format).
- **Nothing changes on failure**: on every exit 1 the Notebook file is unchanged (or still absent) and an existing export target is intact; the atomic writes already guarantee no partial file and no leftover temporary file.
- **Empty or missing Notebook on `export`**: the usual success line, `Exported 0 notes to <PATH>`, exit 0. Rejected: a distinct message (a special case for nothing).

## Verified facts

- clap prints usage errors as `error: …` plus usage lines and exits 2 (decided with the command-line surface).
- The no-notes messages of `list`/`search` (`No notes.`, `No notes tagged <tag>.`) are not errors: exit 0, decided with the list and search output.
- The success line of `export` is `Exported N notes to <PATH>` (`1 note` when N is 1), path as typed.
- A Notebook with a `version` other than 1 is refused; a missing file is an empty Notebook; only `add` creates the file and its parent directories (Notebook format).

## Risks

- A rare `io::ErrorKind` prints a vague text (`other error`); its wording could change between Rust versions. A test would catch it, not a user.
- The binary test for the closed pipe depends on the pipe buffer size: it needs enough output (a large Notebook) to fill the buffer before the read end closes.
- Permission-based tests (unreadable Notebook, unwritable target) do not fail as root; CI must not run them as root.

## Deferred

None.

## Open threads

None.
