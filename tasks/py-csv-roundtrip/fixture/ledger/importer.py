"""Reading CSV reports back."""

from datetime import date
from decimal import Decimal
from typing import List

from .model import Transaction


def parse_row(line: str) -> List[str]:
    """Split one line into fields, honouring double-quoted fields."""
    fields, current, quoted = [], "", False
    for ch in line:
        if ch == '"':
            quoted = not quoted
        elif ch == "," and not quoted:
            fields.append(current)
            current = ""
        else:
            current += ch
    fields.append(current)
    return fields


def read_report(path: str) -> List[Transaction]:
    """Every transaction in the report at `path`."""
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    out = []
    for line in lines[1:]:
        if not line:
            continue
        day, payee, memo, amount = parse_row(line)
        out.append(Transaction(date.fromisoformat(day), payee, memo, Decimal(amount)))
    return out
