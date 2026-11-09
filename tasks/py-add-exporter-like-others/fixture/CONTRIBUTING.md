# Contributing

## Formats

Every format is an `Exporter` subclass in `reportkit/exporters/`, one module
per format, and is listed wherever the existing formats are listed. Give each
its IANA media type.

Text formats follow the same rules:

- The first line names the columns, in the order given.
- A missing value (`None`) is written as an empty field.
- Dates are ISO 8601 (`2026-11-09`); numbers are written as they are, with no
  rounding or thousands separators.
- Lines end with `\n`.
- A format that can't quote its fields (TSV, for one) escapes instead: a
  backslash becomes `\\`, a tab `\t`, a line feed `\n` and a carriage return
  `\r`, each written as two characters.

## Changes

Every change a user would notice gets a line in `CHANGELOG.md` under
`## Unreleased`, in the section that fits (`### Added`, `### Changed`,
`### Fixed`). Every format has its own test module, `tests/test_<format>.py`.
