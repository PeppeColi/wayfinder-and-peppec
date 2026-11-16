# End-to-end tests

**End-to-end tests per feature: on.** `PEPPEC_E2E_FEATURE` is set: a specification decides them in its "End-to-end" section. Kept up to date by `peppec init`.

How this repository writes and runs its end-to-end tests. Planning sessions read it when a
specification decides whether a feature needs new ones (the "End-to-end" section, see the PEPPEC
section of CLAUDE.md); the agent reads it when a work unit writes them.

When they run: on the pull request of a finished feature into the base branch. The tests of the
feature (`PEPPEC_E2E_FEATURE`, skipped when its specification says `None`) and the whole suite
(`PEPPEC_E2E_ALL`). Not on the pull request into `main`.

## Kind

Command line: each test runs the built `notes` binary as a child process and checks stdout,
stderr, exit code and files. An end-to-end test walks a journey across several commands against
one Notebook (for example: add tagged notes at different instants, then `list`, `search`,
`export`). Single-command details belong to the binary tests of the work units
(`tests/add.rs`, `tests/list.rs`, `tests/search.rs`, `tests/export.rs`, `tests/cli.rs`).

## Tool

`assert_cmd` with `predicates` and `tempfile`, in the test target `tests/e2e.rs`. That target is
declared in `Cargo.toml` with `test = false`:

```toml
[[test]]
name = "e2e"
path = "tests/e2e.rs"
test = false
```

Plain `cargo test` does not run it, so work units can merge before a feature's journeys pass;
`cargo build --all-targets` and `cargo clippy --all-targets` still compile and lint it. Do not move
the file or remove `test = false`.

## How the tests of one feature are selected

By name: every end-to-end test is a function named `spec_<N>_<scenario>`, where N is the number of
the specification (for example `spec_12_add_then_search_then_export`).

- `PEPPEC_E2E_FEATURE`: `cargo test --test e2e spec_{spec}_`
- `PEPPEC_E2E_ALL`: `cargo test --test e2e`

## How the application is started for them

No server and no setup: `assert_cmd` runs the `notes` binary that Cargo builds for the test.
Every test builds its command with the shared helper in `tests/common/mod.rs`, which clears the
environment and sets:

- `HOME` and `XDG_DATA_HOME` to a fresh temporary directory (so nothing touches the real data
  directory);
- `NOTES_NOTEBOOK` to a file in that directory;
- `TZ=UTC`;
- `NOTES_NOW` to a fixed RFC 3339 instant (for example `2026-11-15T21:00:00Z`). It is the clock
  override: every command of that process sees exactly that instant. Change it between commands to
  add notes at different instants.

A test overrides or removes single variables when the scenario needs it.
