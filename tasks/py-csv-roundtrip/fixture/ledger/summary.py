"""Figures about a report without loading it all."""


def count_rows(path: str) -> int:
    """How many transactions the report at `path` holds."""
    with open(path, encoding="utf-8") as f:
        return max(sum(1 for line in f if line.strip()) - 1, 0)
