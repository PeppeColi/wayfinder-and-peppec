# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The agent adds one line under "Unreleased" in each pull request; when you release, move those
lines under a new heading with the version and the date.

## [Unreleased]

### Added

- `notes add "<text>"` saves a note to the Notebook, a JSON file in your data directory or wherever `--notebook` / `NOTES_NOTEBOOK` points.
- `notes add -t <TAG>` checks tags (1-32 letters, digits, `-` or `_`), stores them in lowercase without repeats, and rejects an invalid tag without saving anything.
