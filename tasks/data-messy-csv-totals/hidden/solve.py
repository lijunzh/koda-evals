"""Read fixture/data/transactions.csv as an agent should, print October 2026's
totals per category, and what common slips would print instead.

Run from the task directory: python3 hidden/solve.py
"""

import csv
import re
from collections import defaultdict
from datetime import date, datetime
from decimal import Decimal

PATH = "fixture/data/transactions.csv"


def parse_date(text):
    for fmt in ("%Y-%m-%d", "%m/%d/%Y", "%b %d, %Y"):
        try:
            return datetime.strptime(text.strip(), fmt).date()
        except ValueError:
            pass
    return None


def parse_amount(text):
    t = text.strip().replace("$", "").replace(",", "")
    if t.startswith("(") and t.endswith(")"):
        return -Decimal(t[1:-1])
    return Decimal(t)


def totals(dedupe=True, iso_only=False, refunds_as_spend=False, raw_categories=False):
    with open(PATH, newline="", encoding="utf-8-sig") as f:
        rows = list(csv.DictReader(f))
    seen, out = set(), defaultdict(Decimal)
    for r in rows:
        key = tuple(r.values())
        if dedupe and key in seen:
            continue
        seen.add(key)
        if iso_only and not re.match(r"\d{4}-\d\d-\d\d$", r["Date"]):
            continue
        d = parse_date(r["Date"])
        if d is None or (d.year, d.month) != (2026, 10):
            continue
        amount = parse_amount(r["Amount"])
        if refunds_as_spend:
            amount = abs(amount)
        cat = r["Category"] if raw_categories else r["Category"].strip().title()
        out[cat] += amount
    return out


right = totals()
print("answer:")
for cat, v in sorted(right.items()):
    print(f"  {cat:<14}{v:>10.2f}")
for name, kw in {
    "duplicates kept": dict(dedupe=False),
    "only ISO dates": dict(iso_only=True),
    "refunds as spend": dict(refunds_as_spend=True),
}.items():
    t = totals(**kw)
    same = [c for c in right if t[c] == right[c]]
    print(f"slip {name}: same figure for {same or 'none'}")
raw = totals(raw_categories=True)
print(f"slip categories as written: {len(raw)} categories instead of {len(right)}")
