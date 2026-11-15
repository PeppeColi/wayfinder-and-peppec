# What does an export file look like?

`notes export <PATH>` writes the whole Notebook to a Markdown file meant for a person to read or
share: a `# Notes` title, then one section per note, most recent first, with a
`## YYYY-MM-DD HH:MM` heading, the text verbatim, and a `Tags:` line when the note has tags. The
target file must not exist unless `--force` is given; its directory must exist. An empty or
missing Notebook still gives a file (title only). Success prints one confirmation line on stderr.
The file is written atomically. Prototype of every case: [export-file-visual.html](export-file-visual.html).

```
$ TZ=Europe/Rome notes export notes.md
Exported 3 notes to notes.md            (stderr; stdout is empty)

$ cat notes.md
# Notes

## 2026-11-15 22:00

Buy milk

Tags: home, shopping

## 2026-11-15 21:30

Call Anna

## 2026-11-14 09:05

first line
second line

Tags: work
```

## Terms

- **Export**: a copy of the notes written to a file the user chooses. _Avoid_: dump, backup.
- **Notebook**: all the notes of the user, taken together; there is exactly one.
  _Avoid_: store, database.
- **Note**: a piece of text saved by the user, with the instant it was created and zero or more
  tags. _Avoid_: entry, memo, record.
- **Tag**: a short label attached to a note; tags differing only in capitalisation are the same
  tag. _Avoid_: label, category, keyword.

## Why

Wayfinder ticket of the Notes CLI v1 map: decide in what format notes are exported, in what
order, whether an export can be limited to some tags, what happens when the target file exists
or its directory does not, and what `export` does with an empty or missing Notebook. Every rule
must be checkable by an automated test comparing the file, stdout and stderr exactly.

## Locked decisions

### Markdown format

The export is a Markdown file. The glossary says an Export is a copy and not a backup: it is for
a person to read or share. The Notebook file itself (pretty JSON) already serves programs.

| Option | User experience | Correctness | Risk | Cost |
|---|---|---|---|---|
| **A. Markdown** | good | good | mixed | good |
| B. JSON, same notes as the Notebook | mixed | mixed | mixed | good |
| C. Plain text, the `list` lines | mixed | good | good | good |
| D. Several formats via `--format` or extension | good | mixed | poor | good |

- Weakest axis of A, risk: Markdown has no single standard, so the spec fixes every character
  (see "Exact layout" below) for tests to compare exactly.
- B lost: it duplicates the Notebook file; copying that file already gives it.
- C lost: it adds nothing over `notes list > file`.
- D lost: it multiplies tests and spec for a v1.

### One section per note: heading, text, `Tags:` line

Each note is a level-2 heading with its creation instant, the text as a paragraph, then, only if
the note has tags, a line `Tags: a, b`. A note with no tags has no `Tags:` line.

| Option | User experience | Correctness | Risk | Cost |
|---|---|---|---|---|
| **A. Heading, text, `Tags:` line** | good | good | good | good |
| B. One bullet per note, like a `list` line | mixed | mixed | mixed | good |
| C. Heading, tags as `#hashtags` | good | good | mixed | good |

- A is best or tied on every axis.
- B lost: Markdown bullets need indented continuation lines, so multi-line text breaks it.
- C lost: `#home` at the start of a line is a heading in plain Markdown.

## Routine choices

- **Existing target file**: refused with an error and a non-zero exit, nothing written, unless
  `--force` is given; with `--force` the file is replaced. This adds the `--force` flag to
  `export`.
- **No tag filter**: v1 always exports the whole Notebook. A `--tag` option could be added later
  without breaking anything.
- **Order**: same as `list`: by creation instant, most recent first; at equal instants the note
  added later comes first.
- **Missing directory**: error, nothing written; `export` never creates directories (like `cp`).
- **Header**: the file starts with `# Notes` and a blank line. No export date, so two exports of
  the same Notebook are byte-identical.
- **Times**: `YYYY-MM-DD HH:MM`, local time from `TZ`, the same as `list`.
- **Empty or missing Notebook**: the file is written with only the header (`# Notes` plus one
  newline), a message goes to stderr, exit 0. The `--force` rule still applies. A missing
  Notebook stays missing: `export` never creates it.
- **Text verbatim**: the text is written exactly as stored, line breaks kept, no Markdown
  escaping.
- **Success output**: stdout empty; one line on stderr `Exported N notes to <PATH>` (`1 note`
  when N is 1), with the path as the user typed it, exit 0.
- **Exact layout**: `# Notes`, blank line; each note `## <date> <time>`, blank line, the text,
  and if it has tags a blank line and `Tags: a, b` (tags as stored: lowercase, stored order,
  separated by `, `); exactly one blank line between notes; no trailing spaces; the file ends
  with exactly one newline. No `---` rules between notes.
- **Atomic write**: written to a temporary file in the target directory, then renamed over the
  target, as the Notebook is. A failed `--force` leaves the old export intact; no temporary file
  is left behind.

## Verified facts

- `list` and `search` print `YYYY-MM-DD HH:MM` local time from `TZ`, sorted most recent first
  with later-added first on ties (`docs/list-search-output-design.md`).
- The Notebook is written atomically, and only `add` creates it and its parent directories
  (map decision "Where and in what format is the Notebook saved?").
- Tags are 1–32 ASCII letters, digits, `-`, `_`, stored lowercase (`docs/tag-rules-design.md`).

## Risks

- A note line starting with `## ` renders as a new note heading. The file text is still correct;
  only the rendering blurs.
- A note containing a line `---` right under a text line renders that line as a heading.

## Deferred

None.

## Open threads

- Exact wording of the error messages (existing file without `--force`, missing directory, target
  not writable) and of the empty-Notebook stderr message, plus the exit codes: left to the
  error-messages work still in the map's fog.
