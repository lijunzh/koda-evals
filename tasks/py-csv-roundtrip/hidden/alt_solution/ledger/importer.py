"""Reading CSV reports back (a hand-rolled parser over the whole file)."""

from datetime import date
from decimal import Decimal
from typing import Iterator, List

from .model import Transaction


def records(text: str) -> Iterator[List[str]]:
    """RFC 4180 records: quoted fields may hold separators, quotes and line breaks."""
    fields, cur, i, quoted, started = [], [], 0, False, False
    while i < len(text):
        ch = text[i]
        if quoted:
            if ch == '"' and text[i + 1:i + 2] == '"':
                cur.append('"')
                i += 1
            elif ch == '"':
                quoted = False
            else:
                cur.append(ch)
        elif ch == '"':
            quoted, started = True, True
        elif ch == ",":
            fields.append("".join(cur))
            cur, started = [], True
        elif ch in "\r\n":
            if ch == "\r" and text[i + 1:i + 2] == "\n":
                i += 1
            if started or cur:
                fields.append("".join(cur))
                yield fields
            fields, cur, started = [], [], False
        else:
            cur.append(ch)
            started = True
        i += 1
    if started or cur:
        fields.append("".join(cur))
        yield fields


def read_report(path: str) -> List[Transaction]:
    with open(path, encoding="utf-8", newline="") as f:
        rows = list(records(f.read()))
    return [Transaction(date.fromisoformat(d), p, m, Decimal(a)) for d, p, m, a in rows[1:]]
