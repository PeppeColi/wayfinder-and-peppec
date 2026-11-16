# Ping command: design

`notes ping` prints `pong` on one line and exits 0. The option `-n, --times <N>` repeats it N times on the same line, separated by single spaces, with a trailing newline: `notes ping -n 5` prints `pong pong pong pong pong`. N is an integer from 1 to 1000, default 1; any other value is a clap usage error (exit 2). `ping` never reads or writes the Notebook. Binary tests in `tests/ping.rs` cover it; it needs no end-to-end tests.

```
$ notes ping
pong
$ notes ping -n 5
pong pong pong pong pong
$ notes ping --times 3
pong pong pong
$ notes ping -n 0
error: invalid value '0' for '--times <N>': 0 is not in 1..=1000   (clap, with its usage lines; exit 2)
$ NOTES_NOTEBOOK=/path/to/broken.json notes ping
pong                                                               (exit 0: the Notebook is not touched)
```

## Terms

- **ping**: the subcommand of `notes` that prints `pong`, repeated N times with `-n N`; it neither reads nor writes the Notebook. _Avoid_: health check, echo.
- **Notebook**: all the notes of the user, taken together. There is exactly one. _Avoid_: store, database, collection.

## Why

The owner asked for a `ping` feature: calling `notes ping` answers `pong`, and `-n` multiplies the number of `pong` printed, so `notes ping -n 5` returns `pong pong pong pong pong`. PEPPEC's agent cannot run the program: every behaviour below is checkable by a binary test that compares stdout, stderr and the exit code.

## Locked decisions

None. Every choice below is cheap to revisit; raising the upper bound of N later breaks nothing.

## Routine choices

- **Output shape**: one line on stdout, `pong` repeated N times joined by one space, then `\n` (`pong pong pong pong pong\n` for N = 5; `pong\n` for N = 1). stderr empty, exit 0. Rejected: one `pong` per line (contradicts the owner's example); no trailing newline (breaks the prompt and pipes, unlike every other output of `notes`).
- **Option name**: `-n, --times <N>`, on the `ping` subcommand only. Rejected: `-n, --count <N>`; `-n <N>` with no long form (every other option of `notes` has both forms, as `-t, --tag <TAG>`).
- **Valid N**: an integer from 1 to 1000; without the option, N is 1. Validated by clap (`value_parser` with the range `1..=1000`, type `u16` or `u32`): 0, negative numbers, numbers above 1000 and non-numbers produce clap's usage error with its usage lines, exit 2, stdout empty, consistent with the failure contract (clap usage errors keep clap's output and exit 2). Rejected: no upper bound up to `u32::MAX` (a typo can print gigabytes); accepting 0 as an empty line (a special case for nothing); our own `error: invalid count …` message with exit 1 (adds a catalogue entry for what is a usage error).
- **Notebook**: `ping` never reads, creates or writes the Notebook or its directory. The global `--notebook` option and the `NOTES_NOTEBOOK` variable are accepted and ignored: `ping` prints `pong` and exits 0 even when they point to a missing, unreadable or invalid file, or when no data directory exists. Rejected: a health check that reads the Notebook and fails with the catalogue's Notebook errors (a meaning the owner did not ask for, tying `ping` to the whole failure order).
- **Output writing**: like every command, through a writer whose errors are handled, never `println!`: a closed stdout pipe stops quietly with exit 0; any other write error is `error: cannot write output: <reason>`, exit 1 (existing failure contract).
- **Help text**: the subcommand's description is `Reply with pong`; the option's is `Number of pongs, 1-1000` with clap's `[default: 1]`.
- **End-to-end**: `None: unit tests cover it.` `ping` is a single command with no Notebook and no journey across commands; binary tests in `tests/ping.rs` cover output, the range of N and the untouched Notebook. Rejected: a journey `ping` → `add` → `list` (a `size: L` work unit for what one binary test already checks).

## Scenarios (for the specification)

- Given no option, when the user runs `notes ping`, then stdout is `pong\n`, stderr is empty and the exit code is 0.
- Given `-n 5`, when the user runs `notes ping -n 5`, then stdout is `pong pong pong pong pong\n` and the exit code is 0.
- Given `--times 3`, when the user runs `notes ping --times 3`, then stdout is `pong pong pong\n`.
- Given `-n 1000`, when the user runs it, then stdout holds 1000 `pong` separated by single spaces and one trailing newline.
- Given `-n 0`, `-n 1001`, `-n -1` or `-n abc`, when the user runs it, then stdout is empty, stderr starts with `error:` and the exit code is 2.
- Given `NOTES_NOTEBOOK` pointing to a file with invalid JSON, when the user runs `notes ping`, then stdout is `pong\n`, the exit code is 0 and the file is unchanged.
- Given `NOTES_NOTEBOOK` pointing to a path that does not exist, when the user runs `notes ping`, then the file and its parent directory still do not exist afterwards.
- Given `notes --help`, then the list of commands shows `ping` with `Reply with pong`; given `notes ping --help`, then it shows `-n, --times <N>`.

## Verified facts

- The binary is `notes`, built with clap; `src/main.rs` is still a placeholder.
- Failure contract (`docs/failures-design.md`): clap usage errors keep clap's output and exit 2; other failures print one line `error: <message>` and exit 1; a closed stdout pipe exits 0 quietly.
- `--notebook` and `NOTES_NOTEBOOK` are global; binary tests set `NOTES_NOTEBOOK` inside a temporary directory (`docs/testing-design.md`).
- End-to-end tests walk journeys across several commands; single-command details belong to binary tests `tests/<command>.rs` (`docs/agents/e2e.md`).

## Risks

- clap's exact wording of the range error can change between clap versions: tests should check the `error:` prefix and the exit code, not the whole line.

## Deferred

None.

## Open threads

None.
