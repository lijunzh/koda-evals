"""Figures about a report."""

from .importer import records


def count_rows(path: str) -> int:
    with open(path, encoding="utf-8", newline="") as f:
        return max(sum(1 for _ in records(f.read())) - 1, 0)
