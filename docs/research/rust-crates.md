# Rust crates for Notes CLI v1

Research for issue #3 of the map "Notes CLI v1" (#2). Question: which crates to use for
argument parsing, the per-user data directory, end-to-end tests of the binary, storing the
Notebook, and date-times with a clock that tests control.

All versions, dates, MSRVs and download counts come from the crates.io API
(`https://crates.io/api/v1/crates/<name>`), read on 2026-11-15. Behavioural claims were checked
in the published crate source (`https://static.crates.io/crates/<name>/<name>-<version>.crate`)
unless a different source is cited. The project uses Rust edition 2024 (needs Rust 1.85 or
newer); the local toolchain is rustc 1.98.1.

## Summary

| Area | Recommended | Version (released) | MSRV |
|---|---|---|---|
| Arguments, subcommands, `--help` | `clap` with `derive` | 4.6.7 (2026-09-14) | 1.85 |
| Data directory | `dirs` | 7.0.0 (2026-09-05) | none declared |
| End-to-end tests | `assert_cmd` + `predicates` + `tempfile` (or `assert_fs`) | 2.2.2 / 3.1.4 / 3.27.0 (1.1.4) | 1.85 / 1.74 / 1.63 (1.85) |
| Notebook file | `serde` + `serde_json` | 1.0.229 / 1.0.154 | 1.56 / 1.71 |
| Date-times | `jiff` (with `serde` feature) | 0.2.46 (2026-11-15) | 1.70 |

No crate in this list controls the clock or the data directory by itself: both overrides are
something the program must offer (an environment variable, a flag, or a `now` parameter). The
facts below are what that decision needs.

## 1. Argument parsing: `clap`

- **Recommended:** `clap` 4.6.7, features `derive` (and optionally `env`, `wrap_help`).
  Repository: https://github.com/clap-rs/clap. 291 M downloads in the last 90 days; released
  2026-09-14, MSRV 1.85.
- **What it gives:** subcommands as a Rust `enum` with `#[derive(Subcommand)]`, generated
  `--help`, `--version`, usage errors. Doc comments on fields become the help text.
- **Fact for later decisions:**
  - Usage errors exit with code **2** and print to stderr; `--help` and `--version` print to
    stdout and exit **0** (`clap::Error::exit`, docs.rs/clap/4.6.7). The error-codes ticket
    should build on this rather than fight it.
  - The `env` feature lets an argument fall back to an environment variable
    (`#[arg(long, env = "NOTES_DIR")]`). One declaration then gives both a flag and an
    environment override of the Notebook location, and `--help` shows the variable.
  - The parser is testable without running the binary: `Cli::try_parse_from([...])` returns a
    `Result`, so unit tests can check parsing and help text.
- **Alternatives:**
  - `argh` 0.1.19 (2026-03-16, Google): derive-based, small, but follows Fuchsia conventions,
    help text is minimal, still 0.1.
  - `bpaf` 0.9.29 (2026-11-12): combinator or derive API, active, much smaller user base
    (2.8 M recent downloads).
  - `lexopt` 0.3.2 (2026-02-28): a minimal hand-written-loop lexer; no generated help.
  - `pico-args` 0.5.0: last release 2022-06-04; no generated help. Not recommended.

## 2. Data directory: `dirs`

- **Recommended:** `dirs` 7.0.0, released 2026-09-05. 112 M recent downloads. The repository
  moved to Codeberg: https://codeberg.org/dirs/dirs-rs. README badge: "actively developed".
  Depends only on `dirs-sys` 0.5.
- **Locations of `dirs::data_dir()`** (from `src/lin.rs` and `src/mac.rs` of 7.0.0):
  - Linux: `$XDG_DATA_HOME` if set **and absolute**, otherwise `$HOME/.local/share`.
  - macOS: `$HOME/Library/Application Support`. macOS ignores `XDG_*` variables.
  - `home_dir()` (in `dirs-sys` 0.5.0) reads `$HOME` first and falls back to the passwd
    database only if `HOME` is unset or empty.
  - The app appends its own folder, e.g. `data_dir()/notes-cli/`. The function returns
    `Option`: `None` must be handled (no home directory).
- **Fact for later decisions: how a test points it at a temporary directory.**
  - Without any code for it, a test can set `HOME=<tempdir>` and remove `XDG_DATA_HOME` on the
    child process. That redirects the location on both macOS and Linux, because both read
    `$HOME`. It works, but the path differs between the two systems, so expected paths in
    tests become platform-specific.
  - Cleaner: the program reads its own override first (a `--notebook <path>` flag and/or a
    `NOTES_...` environment variable, e.g. through clap's `env` feature) and only falls back to
    `dirs::data_dir()`. Tests then set one variable, the same on both systems. The fallback can
    still be tested once with `HOME`.
  - Environment variables should be set on the child `Command` (see section 3), never with
    `std::env::set_var` in the test process: in edition 2024 `set_var` is `unsafe`, because it
    races with other test threads.
- **Version 7 breaking change:** only `preference_dir` on Windows (README, section "7").
  Irrelevant here.
- **Alternatives:**
  - `directories` 6.0.0 (2025-01-12): the sister crate, gives `ProjectDirs::from(qualifier,
    org, app)`. Its macOS data path uses a bundle-style name
    (`~/Library/Application Support/com.Org.App`). More than a one-user CLI needs.
  - `etcetera` 0.11.0 (2025-10-28, MSRV 1.87): lets you choose the XDG layout on macOS too
    (`~/.local/share`), which some CLI tools prefer. Useful if the map decides on one layout
    for both systems.
  - `xdg` 3.0.0: Unix-only XDG resolution.

## 3. End-to-end tests of the binary: `assert_cmd` + `predicates` + `tempfile`

- **Recommended:** `assert_cmd` 2.2.2 (2026-05-11, MSRV 1.85), `predicates` 3.1.4 (2026-02-11),
  `tempfile` 3.27.0 (2026-03-11). The first two are from the assert-rs organisation
  (https://github.com/assert-rs), which also maintains `snapbox` and `trycmd`, all released in
  2026. As `[dev-dependencies]` only.
- **What they give:**
  - `cargo_bin_cmd!("notes")` (or `Command::cargo_bin("notes")`) runs the binary Cargo built
    for the integration test; it resolves `CARGO_BIN_EXE_<name>`, which Cargo sets only for
    integration tests and benchmarks (`src/macros.rs`).
  - `.env(k, v)`, `.env_remove(k)`, `.env_clear()`, `.current_dir(..)`, `.write_stdin(..)`
    configure the child process; this is how a test sets the Notebook location and the clock
    without touching its own environment (`src/cmd.rs`).
  - `.assert().success()` / `.failure()` / `.code(n)` / `.stdout(pred)` / `.stderr(pred)`
    check exit code and output; `predicates::str::contains`, `is_match`, `diff` build the
    checks.
  - `tempfile::TempDir` (or `assert_fs::TempDir` 1.1.4, which adds `.child("x").assert(..)`
    file checks) gives a directory deleted when the test ends.
- **Fact for later decisions:** a full scenario is "create a `TempDir`, run the binary with
  `.env("<NOTEBOOK_VAR>", dir)` (and the clock variable), assert exit code and stdout, read
  the exported file". Every given/when/then scenario of the map can be written this way.
  The variable names are the program's choice.
- **Alternatives:**
  - `trycmd` 1.2.1 / `snapbox` 1.2.2: snapshot tests where each case is a `.toml` or `.md`
    file with the command and expected output. Good for many help-text cases, heavier to set up.
  - `insta` 1.49.0 with `insta-cmd`: snapshot testing; reviews need the `cargo insta` tool.
  - Plain `std::process::Command` with `env!("CARGO_BIN_EXE_notes")`: no dependency, more
    boilerplate.

## 4. Storing the Notebook: `serde` + `serde_json` (SQLite as comparison)

- **Recommended for a structured file:** `serde` 1.0.229 (2026-07-18, `derive` feature) and
  `serde_json` 1.0.154 (2026-10-11, MSRV 1.71). Both are maintained in the serde-rs
  organisation and have about 435 M recent downloads each.
- **Why JSON over TOML for the Notebook:** the Notebook is a list of records written by the
  program, not a file a human edits. JSON maps a `Vec<Note>` directly; TOML cannot have an
  array at the top level, so the file needs a wrapper table (`[[notes]]`). That wrapper is
  also a natural place for a format version field, which the map lists as not yet specified.
- **TOML option:** `toml` 1.2.1+spec-1.1.0 (2026-11-06, MSRV 1.85), the toml-rs project. Now
  1.x and follows TOML spec 1.1. Fine if a human-editable file becomes a goal.
- **Fact for later decisions:** with a single file, an add means "read all, append, write all".
  For one user without concurrent access this is acceptable. Writing to a temporary file in
  the same directory and renaming it over the old one avoids a half-written Notebook if the
  process dies; `tempfile::NamedTempFile::persist` does exactly that.
- **SQLite comparison:** `rusqlite` 0.43.0, released 2026-11-15 (today), MSRV **1.89**, license
  MIT. With the `bundled` feature it compiles SQLite from C source, so builds need a C
  compiler and take longer; without it, it links the system `libsqlite3`. It gives indexed
  queries and atomic writes, which v1 does not need: list and search over a single user's
  notes fit in memory. Recommendation: a JSON file for v1; SQLite only if a later version
  needs it.

## 5. Date-times and a controllable clock: `jiff`

- **Recommended:** `jiff` 0.2.46 (2026-11-15), by Andrew Gallant (BurntSushi),
  https://github.com/BurntSushi/jiff. 120 M recent downloads, MSRV 1.70, license
  Unlicense OR MIT. Feature `serde` for (de)serialising `Timestamp`.
- **What it gives:** `Timestamp` (an instant, UTC) for storing when a Note was added;
  `Zoned` and `strftime`-style formatting for display; RFC 3339 / ISO 8601 parsing via
  `"2024-07-11T01:14:00Z".parse::<Timestamp>()`.
- **Maintenance:** still 0.2. The README (0.2.46) says 1.0 has no timeline and that 0.2 will
  get critical fixes for one year after 1.0. Breaking changes are therefore possible, but
  0.2 is the version everyone uses today.
- **Fact for later decisions: the clock.**
  - Neither `jiff`, `chrono` nor `time` offers a mockable clock: `Timestamp::now()`,
    `Utc::now()` and `OffsetDateTime::now_utc()` all read the system time.
  - So the program must take "now" from outside: inside the code, the command functions
    receive `now: Timestamp` as a parameter (unit tests pass a fixed value); at the binary
    boundary, `main` reads `Timestamp::now()` unless an override is present (e.g. an
    environment variable holding an RFC 3339 timestamp that `jiff` parses). End-to-end tests
    set that variable on the child process. Which of these the map adopts is the testing
    ticket's decision.
  - Displayed local times depend on the time zone. `jiff` reads the `TZ` environment variable
    first (`src/tz/system/mod.rs`), so tests can set `TZ=UTC` on the child process to get
    stable output. Storing `Timestamp` (UTC) and converting only for display keeps the stored
    data independent of the zone.
- **Alternatives:**
  - `chrono` 0.4.45 (2026-06-04, MSRV 1.62): the long-standing choice, largest ecosystem
    (243 M recent downloads), still 0.4. Works equally well with the same "inject now"
    approach.
  - `time` 0.3.55 (2026-08-01, MSRV 1.88): smaller; local offset on Unix is limited
    (`local-offset` feature) because reading it is unsound in multi-threaded programs.

## Verified / not verified

- Verified in crate source: `dirs` 7.0.0 data-dir rules and `HOME` lookup in `dirs-sys`
  0.5.0; `assert_cmd` 2.2.2 `cargo_bin_cmd!`, `env`, `env_remove`, `env_clear`; `jiff`
  0.2.46 reads `TZ`, README on 1.0.
- From crates.io metadata: all versions, dates, MSRVs, licences, download counts, features.
- From docs, not re-run here: clap's exit codes (2 for usage errors, 0 for help/version),
  `try_parse_from`; `NamedTempFile::persist`; TOML's top-level table rule; `time`'s
  `local-offset` limitation.
- Not done: no crate was compiled or tried in this repository (wayfinder sessions carry no
  production code).
