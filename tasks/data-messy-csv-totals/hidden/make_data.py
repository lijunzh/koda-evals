"""Write fixture/data/ for this task and print the answer.

Run from the task directory: python3 hidden/make_data.py

The answer is checked against the agent's final message. hidden/solve.py
reads the CSV back independently and shows what common slips would give.
"""

import csv
import io
import pathlib
import random
from collections import defaultdict
from datetime import date, timedelta
from decimal import Decimal

random.seed(20261108)

CATEGORIES = {
    "Groceries": (["Corner Market", "FreshCo", "Green Basket", "Harbor Foods"], 12, 180),
    "Dining": (["Noodle Bar", "Taqueria Sol", "The Daily Grind", "Pizza Uno"], 6, 95),
    "Transport": (["Metro Card", "RideShare", "Fuel Stop", "City Parking"], 3, 70),
    "Utilities": (["Power Co", "Water Dept", "FiberNet"], 40, 220),
    "Shopping": (["Bookshop", "Hardware Hub", "Outdoor Supply", "Electronics Depot"], 8, 650),
    "Travel": (["Airline", "Hotel", "Rail Pass"], 80, 1400),
    "Health": (["Pharmacy", "Clinic Copay", "Dental"], 10, 260),
    "Entertainment": (["Cinema", "Streaming", "Concert Hall"], 8, 140),
}
WEIGHTS = [30, 22, 18, 3, 12, 2, 5, 8]
MONTHS = ["Aug", "Sep", "Oct", "Nov"]


def money(amount: Decimal) -> str:
    """The bank's mixed amount formats. Negative amounts are refunds."""
    style = random.random()
    if amount < 0:
        return f"({-amount:.2f})" if style < 0.5 else f"{amount:.2f}"
    if amount >= 1000 and style < 0.7:
        return f"${amount:,.2f}"
    if style < 0.3:
        return f"${amount:.2f}"
    return f"{amount:.2f}"


def day_text(d: date) -> str:
    style = random.random()
    if style < 0.6:
        return d.isoformat()
    if style < 0.85:
        return f"{d.month:02d}/{d.day:02d}/{d.year}"
    return f"{MONTHS[d.month - 8]} {d.day}, {d.year}"


def category_text(c: str) -> str:
    style = random.random()
    if style < 0.75:
        return c
    if style < 0.85:
        return c.lower()
    if style < 0.93:
        return c.upper()
    return f" {c} "


def main() -> None:
    rows, seen = [], set()
    start = date(2026, 8, 1)
    n = 0
    while len(rows) < 4800:
        d = start + timedelta(days=random.randrange(0, 122))  # Aug 1 .. Nov 30
        cat = random.choices(list(CATEGORIES), WEIGHTS)[0]
        names, lo, hi = CATEGORIES[cat]
        n += 1
        desc = f"{random.choice(names)} #{random.randint(100, 999)}"
        amount = Decimal(random.randint(lo * 100, hi * 100)) / 100
        if random.random() < 0.04:
            amount = -amount  # a refund
        key = (d, desc, amount)
        if key in seen:
            continue
        seen.add(key)
        rows.append((d, desc, cat, amount))
    rows.sort(key=lambda r: r[0])

    out_rows = []
    for d, desc, cat, amount in rows:
        line = [day_text(d), desc, category_text(cat), money(amount)]
        out_rows.append(line)
        if random.random() < 0.03:
            out_rows.append(list(line))  # exported twice

    buf = io.StringIO()
    w = csv.writer(buf, lineterminator="\r\n")
    w.writerow(["Date", "Description", "Category", "Amount"])
    w.writerows(out_rows)
    total = sum(r[3] for r in rows)
    w.writerow(["TOTAL", "", "", f"{total:.2f}"])
    data = pathlib.Path("fixture/data")
    data.mkdir(parents=True, exist_ok=True)
    (data / "transactions.csv").write_bytes(b"\xef\xbb\xbf" + buf.getvalue().encode() + b"\r\n")
    (data / "README.md").write_text(
        "Card transactions exported from the bank, August to November 2026.\n\n"
        "- Dates written with slashes are month/day/year.\n"
        "- A negative amount, or one in parentheses, is a refund.\n"
        "- The export sometimes repeats a row exactly. A repeated row is the same\n"
        "  transaction exported twice and counts once.\n"
    )

    def totals(rows_used):
        t = defaultdict(Decimal)
        for d, _, cat, amount in rows_used:
            if d.year == 2026 and d.month == 10:
                t[cat] += amount
        return t

    right = totals(rows)
    print("answer (October 2026):")
    for cat in CATEGORIES:
        print(f"  {cat:<14}{right[cat]:>10.2f}")
    print(f"  {'all':<14}{sum(right.values()):>10.2f}")


if __name__ == "__main__":
    main()
