# ledger-export format specification

Version 3. This document is the contract for every format `ledger-export` writes. Where it and the code disagree, the code is wrong. Each exporter section lists the exact bytes it writes; tests are written from this document, not from the implementations.

## Contents

1. Data model
2. Exporters
3. Registry
4. CSV
5. JSON lines
6. TSV
7. Markdown table
8. Fixed width
9. Error handling
10. Versioning and compatibility
11. Glossary
12. History

## 1. Data model

A ledger is an ordered list of entries. Each entry has a `date`, an ISO 8601 calendar date written `YYYY-MM-DD`; a `payee`, free text of any length that may contain any Unicode character, including tabs, newlines and the delimiter of any format; and an `amount_cents`, a signed 64-bit integer number of cents, negative for money going out. Exporters never reorder entries, never drop an entry, and never merge two entries with the same payee. An empty ledger is valid and every exporter must handle it (see each section for what an empty ledger writes).

Amounts are never floating point anywhere in this crate. Where a format shows a decimal amount, it is produced by `entry::decimal`, which writes the integer part, a dot and exactly two digits, with a leading minus sign for negative amounts and no thousands separators.

## 2. Exporters

An exporter implements `exporters::Exporter`: a `name` that users type after `--format`, an `extension` including the leading dot, and `export`, which turns a slice of entries into the complete file contents as a `String`. Exporters are pure: no I/O, no clock, no environment. Each exporter lives in its own module under `src/exporters/`, named after the format (`csv.rs`, `json.rs`, and so on), with unit tests in that module. A new exporter is not done until it is listed in the registry (section 3).

## 3. Registry

`registry::exporters()` returns every exporter. **The list is sorted by exporter `name`, in byte order**, because the CLI prints it as-is in `--help` and users scan it alphabetically. Adding an exporter means inserting it at its sorted position, not appending it. `registry::by_name` looks names up exactly: no case folding, no prefixes, no aliases.

## 4. CSV

Name `csv`, extension `.csv`. RFC 4180: a header row `date,payee,amount`, then one row per entry. Rows end with CRLF. A payee containing a comma, a double quote, CR or LF is wrapped in double quotes, with each double quote doubled. The amount is `entry::decimal`. An empty ledger writes only the header row.

## 5. JSON lines

Name `jsonl`, extension `.jsonl`. One JSON object per entry, each on its own line ending with LF, keys in the order `date`, `payee`, `amount_cents`, no spaces. The amount is the integer number of cents, not a decimal string. Strings are escaped as JSON requires; control characters other than `\n` and `\t` use `\u00XX`. An empty ledger writes an empty string.

## 6. TSV

Name `tsv`. **Extension `.tab`, not `.tsv`**: the downstream importer this format exists for only recognises `.tab` files.

A header row, then one row per entry, columns separated by a single tab, every row (the header included) ending with LF. **The header is upper case: `DATE`, `PAYEE` and `AMOUNT`, tab-separated like every row.** TSV has no quoting, so a payee is escaped instead: **a backslash becomes `\\`, a tab becomes the two characters `\t`, a newline becomes `\n`, and a carriage return becomes `\r`** (each a backslash followed by a letter). Nothing else is changed. **The amount column is the integer number of cents, as in JSON lines, not a decimal.** An empty ledger writes only the header row.

Example, for one entry dated 2026-03-01 to the payee `Rent⇥March` of -150000 cents, where `⇥` stands for a real tab character here and below:

```
DATE⇥PAYEE⇥AMOUNT
2026-03-01⇥Rent\tMarch⇥-150000
```

In the second row, `Rent\tMarch` is the payee after escaping: `Rent`, a backslash, the letter `t`, `March`.

## 7. Markdown table

Name `markdown`, extension `.md`. A GitHub-flavoured table with three columns, headed `Date`, `Payee` and `Amount`, cells padded with one space on each side: `| Date | Payee | Amount |`. **The separator row right-aligns the amount column: `| --- | --- | ---: |`.** Each entry is a row; **a `|` in the payee is written `\|`**, and a newline in the payee is written as `<br>`. The amount is `entry::decimal`. **After the entries comes one total row: `| | **Total** | **<sum>** |`**, where the sum of every entry's cents is formatted with `entry::decimal` and the date cell is empty (a single space between the pipes). Every line ends with LF. An empty ledger writes the header, the separator and a total row of `**0.00**`.

## 8. Fixed width

Name `fixed`, extension `.txt`. For printing on the old line printer. No header. Each entry is one line: the date (10 characters), two spaces, the payee in a field exactly **24 characters** wide, two spaces, then the amount (`entry::decimal`) **right-aligned in a field 12 characters wide**. Widths count Unicode scalar values (`char`s), not bytes. **A payee longer than 24 characters is cut to its first 23 characters followed by `…` (U+2026, one character)**, so the field stays 24 wide; a shorter payee is padded with spaces on the right. A tab or newline in the payee is replaced by a single space before measuring. **Lines end with CRLF**, the printer's line ending. An empty ledger writes an empty string.

## 9. Error handling

Export cannot fail: every entry is representable in every format, which is why `export` returns `String`, not `Result`. Validation of dates and amounts happens when a ledger is parsed, before any exporter sees it. An exporter that meets a malformed date writes it through unchanged rather than guessing.

## 10. Versioning and compatibility

### 10.1 `csv`

Changing any byte this document fixes for `csv` is a breaking change and needs a major version. The reason is specific to its users: consumers include spreadsheet imports that sniff the delimiter from the first row, so the header is fixed. Additions that a careful consumer would ignore (none exist for this format today) would be minor. When in doubt, treat a change as breaking. Every release lists, per format, whether its output changed, and the integration suite compares each format's output for a fixed ledger against a stored golden file, so an accidental change fails CI before it ships.

### 10.2 `jsonl`

Changing any byte this document fixes for `jsonl` is a breaking change and needs a major version. The reason is specific to its users: consumers stream the file line by line, so an object never spans lines. Additions that a careful consumer would ignore (none exist for this format today) would be minor. When in doubt, treat a change as breaking. Every release lists, per format, whether its output changed, and the integration suite compares each format's output for a fixed ledger against a stored golden file, so an accidental change fails CI before it ships.

### 10.3 `tsv`

Changing any byte this document fixes for `tsv` is a breaking change and needs a major version. The reason is specific to its users: the importer is a batch job that rejects any file whose header does not match byte for byte. Additions that a careful consumer would ignore (none exist for this format today) would be minor. When in doubt, treat a change as breaking. Every release lists, per format, whether its output changed, and the integration suite compares each format's output for a fixed ledger against a stored golden file, so an accidental change fails CI before it ships.

### 10.4 `markdown`

Changing any byte this document fixes for `markdown` is a breaking change and needs a major version. The reason is specific to its users: files are pasted into wiki pages and pull request descriptions, where the total row is read first. Additions that a careful consumer would ignore (none exist for this format today) would be minor. When in doubt, treat a change as breaking. Every release lists, per format, whether its output changed, and the integration suite compares each format's output for a fixed ledger against a stored golden file, so an accidental change fails CI before it ships.

### 10.5 `fixed`

Changing any byte this document fixes for `fixed` is a breaking change and needs a major version. The reason is specific to its users: the printer driver counts characters, not bytes, and expects CRLF; a short line jams the feeder. Additions that a careful consumer would ignore (none exist for this format today) would be minor. When in doubt, treat a change as breaking. Every release lists, per format, whether its output changed, and the integration suite compares each format's output for a fixed ledger against a stored golden file, so an accidental change fails CI before it ships.

## 11. Glossary

- **amount**: as used in this document; see the sections above for the exact meaning of amount in each format. Where a format section and this glossary differ, the format section wins.
- **cents**: as used in this document; see the sections above for the exact meaning of cents in each format. Where a format section and this glossary differ, the format section wins.
- **consumer**: as used in this document; see the sections above for the exact meaning of consumer in each format. Where a format section and this glossary differ, the format section wins.
- **CRLF**: as used in this document; see the sections above for the exact meaning of CRLF in each format. Where a format section and this glossary differ, the format section wins.
- **decimal**: as used in this document; see the sections above for the exact meaning of decimal in each format. Where a format section and this glossary differ, the format section wins.
- **delimiter**: as used in this document; see the sections above for the exact meaning of delimiter in each format. Where a format section and this glossary differ, the format section wins.
- **entry**: as used in this document; see the sections above for the exact meaning of entry in each format. Where a format section and this glossary differ, the format section wins.
- **escape**: as used in this document; see the sections above for the exact meaning of escape in each format. Where a format section and this glossary differ, the format section wins.
- **exporter**: as used in this document; see the sections above for the exact meaning of exporter in each format. Where a format section and this glossary differ, the format section wins.
- **extension**: as used in this document; see the sections above for the exact meaning of extension in each format. Where a format section and this glossary differ, the format section wins.
- **field**: as used in this document; see the sections above for the exact meaning of field in each format. Where a format section and this glossary differ, the format section wins.
- **golden file**: as used in this document; see the sections above for the exact meaning of golden file in each format. Where a format section and this glossary differ, the format section wins.
- **header row**: as used in this document; see the sections above for the exact meaning of header row in each format. Where a format section and this glossary differ, the format section wins.
- **ledger**: as used in this document; see the sections above for the exact meaning of ledger in each format. Where a format section and this glossary differ, the format section wins.
- **LF**: as used in this document; see the sections above for the exact meaning of LF in each format. Where a format section and this glossary differ, the format section wins.
- **payee**: as used in this document; see the sections above for the exact meaning of payee in each format. Where a format section and this glossary differ, the format section wins.
- **registry**: as used in this document; see the sections above for the exact meaning of registry in each format. Where a format section and this glossary differ, the format section wins.
- **row**: as used in this document; see the sections above for the exact meaning of row in each format. Where a format section and this glossary differ, the format section wins.
- **scalar value**: as used in this document; see the sections above for the exact meaning of scalar value in each format. Where a format section and this glossary differ, the format section wins.
- **separator row**: as used in this document; see the sections above for the exact meaning of separator row in each format. Where a format section and this glossary differ, the format section wins.
- **total row**: as used in this document; see the sections above for the exact meaning of total row in each format. Where a format section and this glossary differ, the format section wins.
- **width**: as used in this document; see the sections above for the exact meaning of width in each format. Where a format section and this glossary differ, the format section wins.

## 12. History

- 0.0.1: maintenance release 1. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 2 clarified without changing any rule.
- 0.0.2: maintenance release 2. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 3 clarified without changing any rule.
- 0.0.3: maintenance release 3. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 4 clarified without changing any rule.
- 0.0.4: maintenance release 4. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 5 clarified without changing any rule.
- 0.0.5: maintenance release 5. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 6 clarified without changing any rule.
- 0.0.6: maintenance release 6. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 7 clarified without changing any rule.
- 0.0.7: maintenance release 7. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 8 clarified without changing any rule.
- 0.0.8: maintenance release 8. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 9 clarified without changing any rule.
- 0.0.9: maintenance release 9. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 1 clarified without changing any rule.
- 0.1.0: maintenance release 10. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 2 clarified without changing any rule.
- 0.1.1: maintenance release 11. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 3 clarified without changing any rule.
- 0.1.2: maintenance release 12. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 4 clarified without changing any rule.
- 0.1.3: maintenance release 13. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 5 clarified without changing any rule.
- 0.1.4: maintenance release 14. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 6 clarified without changing any rule.
- 0.1.5: maintenance release 15. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 7 clarified without changing any rule.
- 0.1.6: maintenance release 16. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 8 clarified without changing any rule.
- 0.1.7: maintenance release 17. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 9 clarified without changing any rule.
- 0.1.8: maintenance release 18. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 1 clarified without changing any rule.
- 0.1.9: maintenance release 19. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 2 clarified without changing any rule.
- 0.2.0: maintenance release 20. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 3 clarified without changing any rule.
- 0.2.1: maintenance release 21. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 4 clarified without changing any rule.
- 0.2.2: maintenance release 22. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 5 clarified without changing any rule.
- 0.2.3: maintenance release 23. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 6 clarified without changing any rule.
- 0.2.4: maintenance release 24. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 7 clarified without changing any rule.
- 0.2.5: maintenance release 25. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 8 clarified without changing any rule.
- 0.2.6: maintenance release 26. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 9 clarified without changing any rule.
- 0.2.7: maintenance release 27. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 1 clarified without changing any rule.
- 0.2.8: maintenance release 28. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 2 clarified without changing any rule.
- 0.2.9: maintenance release 29. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 3 clarified without changing any rule.
- 0.3.0: maintenance release 30. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 4 clarified without changing any rule.
- 0.3.1: maintenance release 31. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 5 clarified without changing any rule.
- 0.3.2: maintenance release 32. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 6 clarified without changing any rule.
- 0.3.3: maintenance release 33. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 7 clarified without changing any rule.
- 0.3.4: maintenance release 34. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 8 clarified without changing any rule.
- 0.3.5: maintenance release 35. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 9 clarified without changing any rule.
- 0.3.6: maintenance release 36. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 1 clarified without changing any rule.
- 0.3.7: maintenance release 37. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 2 clarified without changing any rule.
- 0.3.8: maintenance release 38. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 3 clarified without changing any rule.
- 0.3.9: maintenance release 39. No output changes in any format. Internal refactoring of exporter jsonl and its tests; documentation wording in section 4 clarified without changing any rule.
- 0.4.0: maintenance release 40. No output changes in any format. Internal refactoring of exporter csv and its tests; documentation wording in section 5 clarified without changing any rule.
