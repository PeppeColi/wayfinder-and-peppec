# Tag rules: design

A tag is 1 to 32 ASCII characters: letters, digits, `-` and `_`. The first character is a letter or a digit. Capitalisation does not matter: a tag is stored in lowercase, and the tag given to `search` is lowercased before matching. A tag repeated on the same note is stored once, silently, in the order of its first occurrence. A note with no tags is valid. An invalid tag is an error in `add` (no note saved, Notebook untouched) and in `search` alike. `search` takes one tag only (decided earlier, with the command-line surface).

This document resolves the wayfinder ticket "What are the rules for tags?" of the map "Map: Notes CLI v1".

## Terms

- **Tag**: a short label attached to a note, used to find it again. Tags that differ only in capitalisation are the same tag. _Avoid_: label, category, keyword.
- **Note**: a piece of text saved by the user, with the instant it was created and zero or more tags. _Avoid_: entry, memo, record.

## Why

The map needs exact rules for tags before the specification can state the scenarios of `add` and `search`: what a valid tag is, how capitalisation is treated, and what happens with repeated, missing and invalid tags. Every rule must be checkable by an automated test.

## Locked decisions

None. Every choice below is easy to revisit: widening the character set or the length limit later keeps every existing Notebook valid.

## Routine choices

- **Allowed characters**: ASCII letters, digits, `-` and `_`. Rejected: any Unicode character except whitespace (opens Unicode normalisation and non-ASCII case folding); any non-empty string (tags with spaces and control characters). Cost: `caffè` is not a valid tag.
- **First character**: a letter or a digit. `-` and `_` only after the first character. Reason: `notes search -x` and `notes add "x" -t -x` would be parsed as options, so a tag starting with `-` could not be typed normally.
- **Length**: 1 to 32 characters. The empty tag (`-t ""`) is invalid.
- **Capitalisation**: case-insensitive. A tag is stored in lowercase in the Notebook. The tag of `search` is lowercased before matching: `notes add "x" -t Home` then `notes search HOME` finds the note, and the Notebook holds `"home"`. Rejected: storing as typed and comparing case-insensitively (the same tag could appear in two spellings); case-sensitive tags.
- **Repeated tag on one note**: deduplicated after lowercasing, silently, keeping the order of first occurrence. `-t b -t a -t B` stores `["b", "a"]`. Rejected: alphabetical order; an error.
- **Note with no tags**: valid. Stored with `"tags": []`; shown by `list`, never matched by `search`.
- **Invalid tag**: an error in every command. In `add`, one invalid tag rejects the whole note: nothing is saved and the Notebook file is unchanged (or still absent). In `search`, an invalid tag is an error, not an empty result. Rejected: no error in `search`; dropping the invalid tag with a warning in `add`.
- **Search with several tags**: not applicable: `search` takes one positional tag (decided with the command-line surface).

## Verified facts

- The command-line surface already fixes tags on `add` as the repeated flag `-t, --tag <TAG>`, and `search <TAG>` as one positional tag.
- The Notebook format already stores `tags` as a JSON array of strings per note.

## Risks

- The error message and exit code for an invalid tag are not decided here: they belong to the error-handling decisions still in the map's fog. Tests written before that decision can only check "non-zero exit, Notebook unchanged".

## Deferred

None.

## Open threads

None.
