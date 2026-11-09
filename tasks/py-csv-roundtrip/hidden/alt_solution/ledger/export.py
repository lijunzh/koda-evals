"""Writing CSV reports (the hand-rolled writer, fixed)."""

from typing import Iterable, List

from .model import Transaction

HEADER = ["date", "payee", "memo", "amount"]
_SPECIAL = (",", '"', "\n", "\r")


def _field(value: str) -> str:
    """Quote a field holding a separator, a quote or a line break, or with
    spaces at either end; double the quotes inside."""
    if any(c in value for c in _SPECIAL) or value != value.strip():
        return '"' + value.replace('"', '""') + '"'
    return value


def format_row(fields: List[str]) -> str:
    return ",".join(_field(f) for f in fields)


def to_fields(t: Transaction) -> List[str]:
    return [t.day.isoformat(), t.payee, t.memo, f"{t.amount:.2f}"]


def write_report(path: str, transactions: Iterable[Transaction]) -> int:
    n = 0
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(format_row(HEADER) + "\r\n")
        for t in transactions:
            f.write(format_row(to_fields(t)) + "\r\n")
            n += 1
    return n
