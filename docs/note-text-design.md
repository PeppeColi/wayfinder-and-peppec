# Note text: design

The text of a note comes only from the `<TEXT>` argument of `notes add`: there is no stdin and no editor. Line breaks are accepted. Before the text is saved, `\r\n` and any lone `\r` become `\n`, then leading and trailing whitespace is removed (`str::trim`); everything inside stays as given, blank lines and indentation included. If nothing is left, the text is empty: an error, no note saved, Notebook unchanged (or still absent). There is no length limit of our own. A text that starts with `-` is given after the standard `--` separator.

This document resolves the wayfinder ticket "How is the text of a note given, and may it span several lines?" of the map "Map: Notes CLI v1".

```
notes add "Buy milk"                     # text: "Buy milk"
notes add $'first\nsecond' -t work       # text: "first\nsecond"
notes add $'  Buy milk \r\n'             # text: "Buy milk"
notes add $'a\r\nb\rc'                   # text: "a\nb\nc"
notes add $'  para one\n\n    indented'  # text: "para one\n\n    indented"
notes add ""                             # error: empty text, nothing saved
notes add $' \t\n'                       # error: empty text, nothing saved
notes add -- "-5 degrees outside"        # text: "-5 degrees outside"
notes add "-5 degrees outside"           # clap usage error, exit 2
echo hi | notes add                      # clap usage error (missing <TEXT>), exit 2
```

## Terms

- **Note**: a piece of text saved by the user, with the instant it was created and zero or more tags. _Avoid_: entry, memo, record.

## Why

The specification needs exact rules for the text that `notes add` saves: where it comes from, whether it may span several lines, and what counts as empty. `list`/`search` already indent continuation lines and `export` already writes the text verbatim, so the input side has to match. Every rule must be checkable by a binary test, because PEPPEC's agent cannot run the program.

## Locked decisions

None. Every choice below is easy to revisit: adding stdin later (`-` as the text) or a length limit later changes only `add`.

## Routine choices

- **Source of the text**: only the required positional `<TEXT>` of `notes add`, as the command-line surface already fixes. No stdin, no `$EDITOR`. To save a file's contents the user writes `notes add "$(cat file)"`. Rejected: stdin when the argument is `-` (cheap, but `-` could never be saved as a text; the natural upgrade later); stdin when the argument is omitted and stdin is not a terminal (depends on TTY detection, behaves differently in tests and cron); `$EDITOR` (needs a fake editor in every test).
- **Line breaks**: accepted and stored. `list`/`search` show continuation lines indented by 18 spaces; `export` writes them verbatim. Rejected: an error (would make those output rules dead code); replacing them with spaces (silently changes the text).
- **Line endings**: `\r\n` and any lone `\r` are replaced by `\n` before saving. Every other character, tabs and other control characters included, is kept as given. Rejected: also rejecting control characters other than `\n` and `\t` (one more error case for pasted text; the risk it closes, an escape sequence printed raw by `list`, is theoretical: only text the user typed, on one machine); keeping `\r` verbatim (garbles `list` lines).
- **Trimming**: after line-ending normalisation, leading and trailing whitespace is removed with Rust's `str::trim` (Unicode White_Space). The inside of the text is never changed: blank lines, indentation and trailing spaces on inner lines stay. Rejected: storing as given (leading spaces break the `list` column, a trailing newline adds a blank line to `export`); also trimming each line's trailing spaces (touches the user's formatting, e.g. Markdown hard breaks).
- **Empty text**: the text is empty when nothing is left after trimming, so `""` and whitespace-only text such as `" \t\n"` are both empty. Empty text is an error: no note saved, Notebook file unchanged or still absent. The exact message and exit code belong to the error-handling ticket. Rejected: rejecting only `""`; accepting empty notes.
- **Processing order**: normalise line endings → trim → empty check → save. Tag validation is independent; an empty text and an invalid tag are both errors with nothing saved.
- **Length**: no limit of our own; the operating system's argument limit is the only bound. Rejected: a fixed limit such as 10,000 characters (an error case and a test for nothing).
- **Text starting with `-`**: given after `--`, e.g. `notes add -t x -- "-5 degrees"`, clap's standard behaviour with no extra code. Without `--`, clap reports a usage error (exit 2). Rejected: `allow_hyphen_values` on `<TEXT>` (a mistyped option would silently become the text of a note).

## Verified facts

- The command-line surface already fixes `notes add [OPTIONS] <TEXT>` with `<TEXT>` required, and tags as the repeated flag `-t, --tag <TAG>`.
- `list`/`search` indent continuation lines by 18 spaces (the width of `YYYY-MM-DD HH:MM  `), tags after the last line; `export` writes the text verbatim, line breaks kept.
- `assert_cmd` passes arguments containing `\n` and `\r` unchanged, so every rule above is testable as a binary test; the normalisation and trimming are also unit-testable as a pure function.

## Risks

- An escape character in a note is printed raw by `list`/`search`. Theoretical: the text comes only from the user's own command line.
- `"$(cat file)"` strips trailing newlines in the shell and is limited by the OS argument size; acceptable for v1.

## Deferred

None.

## Open threads

None.
