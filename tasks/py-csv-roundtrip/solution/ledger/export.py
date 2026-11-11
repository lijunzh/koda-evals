"""Writing CSV reports."""

import csv
from typing import Iterable, List

from .model import Transaction

HEADER = ["date", "payee", "memo", "amount"]


def to_fields(t: Transaction) -> List[str]:
    return [t.day.isoformat(), t.payee, t.memo, f"{t.amount:.2f}"]


def write_report(path: str, transactions: Iterable[Transaction]) -> int:
    """Write `transactions` to `path` as CSV with a header row; return how many.

    The csv module quotes any field holding a comma, a quote or a line break,
    and doubles the quotes inside, so every text survives and spreadsheets
    read it.
    """
    n = 0
    with open(path, "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(HEADER)
        for t in transactions:
            w.writerow(to_fields(t))
            n += 1
    return n
