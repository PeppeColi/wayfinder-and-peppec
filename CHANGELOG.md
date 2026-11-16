# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The agent adds one line under "Unreleased" in each pull request; when you release, move those
lines under a new heading with the version and the date.

## [Unreleased]

### Added

- `notes add "<text>"` saves a note to the Notebook, a JSON file in your data directory or wherever `--notebook` / `NOTES_NOTEBOOK` points.
- `notes add -t <TAG>` checks tags (1-32 letters, digits, `-` or `_`), stores them in lowercase without repeats, and rejects an invalid tag without saving anything.
- `notes add` keeps line breaks inside the text, turns Windows line endings into newlines, trims spaces and blank lines around it, and refuses text that is empty.
- `notes list` prints every note, most recent first, with its local date and time, text and tags, and says `No notes.` on stderr when there are none.
- `notes search <TAG>` prints the notes with that tag (ignoring capitalisation), in the same layout and order as `list`, and says `No notes tagged <tag>.` on stderr when there are none.
- `notes export [--force] <PATH>` writes the whole Notebook to a Markdown file, most recent note first; it refuses to replace an existing file without `--force` and never creates directories.
