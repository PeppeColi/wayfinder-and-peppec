# How do list and search show notes?

`notes list` and `notes search <TAG>` print the same layout: one line per note, local time to
the minute, then the text, then the tags in brackets. Notes are sorted by creation instant,
most recent first; at equal instants the note added later comes first. No limit option, no
colours, no truncation. When nothing matches, stdout stays empty, a short message goes to
stderr, and the exit code is 0.

```
$ TZ=Europe/Rome notes list
2026-11-15 22:00  Buy milk  [home, shopping]
2026-11-15 21:30  Call Anna
2026-11-14 09:05  first line
                  second line  [work]
```

## Terms

- **Note**: a piece of text saved by the user, with the instant it was created and zero or
  more tags. _Avoid_: entry, memo, record.
- **Tag**: a short label attached to a note; tags differing only in capitalisation are the
  same tag. _Avoid_: label, category, keyword.
- **Notebook**: all the notes of the user, taken together; there is exactly one.
  _Avoid_: store, database.

## Why

Wayfinder ticket of the Notes CLI v1 map: decide how list and search print each note, in what
order, how times are shown, and whether output can be limited. Every rule must be checkable by
an automated test comparing stdout and stderr exactly.

## Locked decisions

### One line per note

Layout: `<date> <time>`, two spaces, the text, then, if the note has tags, two spaces and
`[tag1, tag2]` (tags as stored: lowercase, in stored order, separated by `, `). A note with no
tags has no brackets and no trailing spaces. No identifier is printed (no v1 command refers to
a single note).

| Option | User experience | Correctness | Risk | Cost |
|---|---|---|---|---|
| **A. One line per note** | good | good | mixed | good |
| B. Block per note | mixed | good | good | good |
| C. Tab-separated fields | poor | mixed | mixed | good |

- Weakest axis of A, risk: a text with line breaks breaks the one-line shape; covered by the
  multi-line rule below.
- B lost: it doubles the height of every listing.
- C lost: it serves machines, and machines already have `export`.

### Local time, to the minute

Format `YYYY-MM-DD HH:MM` in the local time zone, taken from the `TZ` environment variable
(the time crate jiff reads it). Tests set `TZ` as they set the clock.

| Option | User experience | Correctness | Risk | Cost |
|---|---|---|---|---|
| **A. Local, to the minute** | good | good | mixed | good |
| B. Local, to the second | mixed | good | mixed | good |
| C. UTC as stored | poor | good | good | good |

- Weakest axis of A, risk: output depends on the time zone, so every test must set `TZ`; two
  notes in the same minute show the same time (their order stays exact).
- B lost: seconds add noise without helping recognise a note.
- C lost: the person thinks in local time.

## Routine choices

- **Order**: sort by `created` descending; at equal instants the note added later (later in
  the file) comes first. Holds even when a later `add` carries an earlier instant.
- **`search` output**: same layout and order as `list`, restricted to the notes with the tag.
- **Multi-line text**: each continuation line is indented by 18 spaces (the width of
  `YYYY-MM-DD HH:MM  `), so it lines up under the text; the tags follow the last line.
- **Limit**: no `--limit` in v1; `notes list | head -n 5` does it. A closed pipe must end the
  program quietly; the exact behaviour belongs to the error handling.
- **No notes to show**: stdout empty, exit 0, and on stderr `No notes.` (`list`, empty or
  missing Notebook) or `No notes tagged <tag>.` (`search`, tag shown lowercased).
- **Styling**: plain text, no colours, never truncated to the terminal width.

## Verified facts

- None established by experiment in this session. That jiff takes the system time zone from
  `TZ` comes from its documentation; the testing ticket should confirm it with a test.

## Risks

- If `TZ` handling differs between macOS and Linux runners, tests could pass locally and fail
  in CI: the testing ticket decides how tests set it.
- Rust's `println!` panics on a closed pipe (`notes list | head -n 1`): the error handling must
  cover it.

## Deferred

- None.

## Open threads

- Whether `add` should accept text with line breaks at all: belongs to the note text input
  decision.
