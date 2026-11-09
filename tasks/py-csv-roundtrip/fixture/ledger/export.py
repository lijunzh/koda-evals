"""Writing CSV reports."""

from typing import Iterable, List

from .model import Transaction

HEADER = ["date", "payee", "memo", "amount"]


def _field(value: str) -> str:
    """A field as it goes in the file: quoted when it holds a comma."""
    if "," in value:
        return '"' + value + '"'
    return value


def format_row(fields: List[str]) -> str:
    return ",".join(_field(f) for f in fields)


def to_fields(t: Transaction) -> List[str]:
    return [t.day.isoformat(), t.payee, t.memo, f"{t.amount:.2f}"]


def write_report(path: str, transactions: Iterable[Transaction]) -> int:
    """Write `transactions` to `path` as CSV with a header row; return how many."""
    n = 0
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(format_row(HEADER) + "\n")
        for t in transactions:
            f.write(format_row(to_fields(t)) + "\n")
            n += 1
    return n
