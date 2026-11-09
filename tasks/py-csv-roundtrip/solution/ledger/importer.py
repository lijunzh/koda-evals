"""Reading CSV reports back."""

import csv
from datetime import date
from decimal import Decimal
from typing import List

from .model import Transaction


def read_report(path: str) -> List[Transaction]:
    """Every transaction in the report at `path`.

    `newline=""` hands line breaks inside quoted fields to the csv module
    untouched, instead of letting the file object split on them.
    """
    with open(path, encoding="utf-8", newline="") as f:
        rows = list(csv.reader(f))
    out = []
    for row in rows[1:]:
        if not row:
            continue
        day, payee, memo, amount = row
        out.append(Transaction(date.fromisoformat(day), payee, memo, Decimal(amount)))
    return out
