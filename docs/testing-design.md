# How the behaviours are tested, and how end-to-end tests run

Notes CLI v1 is tested at three levels. **Unit tests** call a library crate (`src/lib.rs`) whose
functions receive the current instant and the time zone as parameters. **Binary tests** run the
built `notes` binary with `assert_cmd` in a clean, temporary environment, on every pull request
(`cargo test`). **End-to-end tests** live in a separate test target that plain `cargo test` does
not run; they walk journeys across several commands and run only at the end of a feature, through
`PEPPEC_E2E_FEATURE` and `PEPPEC_E2E_ALL`. Tests set the clock with the hidden variable
`NOTES_NOW` (one RFC 3339 instant), the time zone with `TZ`, and the Notebook with
`NOTES_NOTEBOOK` inside a temporary directory.

## Terms

- **Unit test**: a Rust test that calls the library code directly, passing the current instant and
  the time zone as parameters; no process. _Avoid_: component test.
- **Binary test**: a test that runs the built `notes` binary as a child process with `assert_cmd`,
  with its own environment, and checks stdout, stderr, exit code and files. Part of every work
  unit, run by `cargo test`. _Avoid_: integration test (ambiguous in Rust).
- **End-to-end test**: a binary test that walks one scenario of a specification across several
  commands, written after the feature's work units, run only by `PEPPEC_E2E_FEATURE` /
  `PEPPEC_E2E_ALL`. _Avoid_: acceptance test, system test.

## Why

PEPPEC's agent cannot run the program: "done" means the checks pass. Every behaviour decided on
the map must therefore be provable by an automated test, including "most recent first" (needs a
controllable clock), local times (needs a controllable time zone) and files written or refused
(needs a Notebook location that is never the developer's real one). The ticket also has to fill in
`docs/agents/e2e.md`: kind, tool, selection of one specification's tests, and how the application
is started.

## Locked decisions

### Separate end-to-end level: a test target outside plain `cargo test`

**Kind: command line. Tool: `assert_cmd`. A separate test target `tests/e2e.rs` declared with
`test = false` in `Cargo.toml`. Each test is named `spec_<N>_<scenario>`, where N is the number of
the specification. `PEPPEC_E2E_FEATURE` = `cargo test --test e2e spec_{spec}_`;
`PEPPEC_E2E_ALL` = `cargo test --test e2e`.**

`test = false` keeps the target out of plain `cargo test` (the preset's check), so work units can
merge before the whole scenario passes; `cargo build --all-targets` and
`cargo clippy --all-targets` in the check still compile and lint it, so it cannot rot. Naming
`--test e2e` explicitly runs it.

| Option | User experience | Correctness | Risk | Cost |
|---|---|---|---|---|
| **A: separate target, `test = false`, name prefix `spec_<N>_`** | – | good | good | good |
| B: no end-to-end level, specifications write `None` | – | mixed | mixed | good |
| C: e2e tests in the default `tests/` targets, also run by `cargo test` | – | mixed | mixed | mixed |

- **B lost**: cheaper, but nothing checks the seams between work units written in different pull
  requests (what `add` writes is what `list`, `search` and `export` read), and PEPPEC's e2e path
  stays unexercised.
- **C lost**: the end-to-end test of a feature would run on every pull request, so it either breaks
  earlier work units or has to be written last anyway; it also runs the same journeys twice.
- Weakest point of A: one more `size: L` work unit per specification, and the `test = false`
  arrangement must be stated in `docs/agents/e2e.md` so the agent does not move the file.

## Routine choices

- **Clock override**: environment variable `NOTES_NOW`, hidden from `--help`, holding one RFC 3339
  instant with offset (`2026-11-15T21:00:00Z`, `2026-11-15T22:00:00+01:00`). Every command in that
  process sees exactly that instant. An invalid value is an error with a non-zero exit, never a
  silent fallback to the real clock. Tests add notes at different instants by changing the value
  between runs. The library takes `now` as a parameter; only `main` reads `NOTES_NOW`, then
  `Timestamp::now()`.
- **Time zone**: the program uses jiff's system time zone, which reads `TZ` (decided in "How do list
  and search show notes?"). Binary tests set `TZ`; unit tests pass the time zone as a parameter.
- **Unit vs binary tests**: library + thin `main`.
  - Unit tests: tag validation and normalisation, ordering (including ties), line formatting
    (with instant and time zone passed in, multi-line indentation), Markdown export rendering,
    Notebook JSON (de)serialisation and the version check.
  - Binary tests: every command's observable contract — stdout, stderr, exit code, files created,
    left untouched or refused — at least one per user-visible acceptance criterion.
- **Isolation of binary tests**: one shared helper in `tests/common/mod.rs` builds the command:
  `env_clear()`, then `HOME` and `XDG_DATA_HOME` inside a fresh tempdir, `NOTES_NOTEBOOK` = a file
  in that tempdir, `TZ=UTC`, `NOTES_NOW` = a fixed instant. Tests override or remove single
  variables; the default-location test removes `NOTES_NOTEBOOK`. At least one test uses
  `TZ=Europe/Rome`. End-to-end tests use the same helper.
- **What end-to-end tests cover**: only journeys that cross commands against one Notebook (e.g. add
  several tagged notes at different instants → `list` order → `search` → `export` file contents).
  Single-command details (each error, each tag rule) stay in the work units' tests.
- **Layout**: one binary test file per command — `tests/add.rs`, `tests/list.rs`,
  `tests/search.rs`, `tests/export.rs` — plus `tests/cli.rs` for global behaviour (`--help`,
  `--notebook`, default location); helper in `tests/common/mod.rs`; unit tests in
  `#[cfg(test)] mod tests` beside the code; end-to-end tests in `tests/e2e.rs`.

## Verified facts

- The `rust-v1` preset's check is `cargo fmt --all -- --check`, `cargo build --all-targets`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` (PEPPEC engine,
  `presets/rust-v1/preset.yml@v2`).
- `PEPPEC_E2E_FEATURE` is a command in which `{spec}` is replaced by the specification's number; it
  runs on the pull request of a finished feature branch into the base branch, as does
  `PEPPEC_E2E_ALL` (engine `scripts/e2e.sh`, `docs/setup.md`). Neither variable is set in this
  repository yet.
- `dirs::data_dir()` reads `HOME` (and `XDG_DATA_HOME` on Linux); with `HOME` unset it falls back to
  the passwd entry, i.e. the real home. Hence the helper always sets `HOME`.
- The default Notebook path differs by OS: `$XDG_DATA_HOME/notes/notebook.json` on Linux (CI),
  `$HOME/Library/Application Support/notes/notebook.json` on macOS; the default-location test picks
  the expectation with `cfg!(target_os = …)`.
- Named zones such as `Europe/Rome` need the system tz database: present on GitHub's Ubuntu runners
  and on macOS.

## Risks

- `NOTES_NOW` ships in production builds. Hidden, and it only changes the stored instant of the
  user's own notes: accepted.
- `test = false` is a convention someone could undo; `docs/agents/e2e.md` states it, and
  `docs/agents` is a protected path for the agent.
- If the helper forgot `HOME`, a binary test could touch the real data directory: the helper is the
  single place that sets it.

## Deferred

None.

## Open threads

- Setting `PEPPEC_E2E_FEATURE` and `PEPPEC_E2E_ALL` on the repository, and requiring
  `ci / e2e_all` on the base branch, are setup steps for the owner, not decisions.
- Tests for error cases (unreadable Notebook, closed pipe, …) follow the error-handling decisions
  still open on the map.
