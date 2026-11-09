# Formats

| Format | Option | Extension | Media type | Notes |
|---|---|---|---|---|
| CSV | `csv` | `.csv` | `text/csv` | Quoted where needed; opens in any spreadsheet. |
| JSON | `json` | `.json` | `application/json` | A list of objects, one per record, keys in column order. |
| TSV (tab-separated) | `tsv` | `.tsv` | `text/tab-separated-values` | Escapes backslash, tab and line breaks. |

All text formats name the columns on the first line, write a missing value as
an empty field and dates as ISO 8601. See `CONTRIBUTING.md` for the rules a new
format follows.
