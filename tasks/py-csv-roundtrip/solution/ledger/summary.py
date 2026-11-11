"""Figures about a report without loading it all."""

import csv


def count_rows(path: str) -> int:
    """How many transactions the report at `path` holds: records, not lines."""
    with open(path, encoding="utf-8", newline="") as f:
        return max(sum(1 for row in csv.reader(f) if row) - 1, 0)
