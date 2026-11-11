"""Write fixture/ for this task: the bank's export and the budget app's
export of the same card, September and October 2026, which disagree in
exactly three places. Run from the task directory: python3 hidden/make_data.py

The three: a charge the budget app never got, a charge it has twice, and one
whose amount it has with two digits swapped. Decoys that agree but look like
they might not: two identical coffees on one day (in both), a refund (a
credit in the bank, an inflow in the budget), and the bank's payees written
the bank's way.
"""

import csv
import io
import pathlib
import random
from datetime import date, timedelta
from decimal import Decimal

random.seed(20261109)

PAYEES = {
    "Corner Market": "CORNER MKT", "FreshCo": "FRESHCO STORES", "Harbor Foods": "HARBOR FOODS",
    "Noodle Bar": "SQ *NOODLE BAR", "Taqueria Sol": "TST* TAQUERIA SOL", "The Daily Grind": "THE DAILY GRIND",
    "Metro Card": "METRO TRANSIT", "Fuel Stop": "FUEL STOP", "Power Co": "POWER CO AUTOPAY",
    "FiberNet": "FIBERNET", "Bookshop": "BOOKSHOP.ORG", "Hardware Hub": "HARDWARE HUB",
    "Pharmacy": "CVS PHARMACY", "Cinema": "CINEMA CITY", "Streaming": "STREAMFLIX.COM",
}
MISSING = (date(2026, 10, 14), "Harbor Foods", Decimal("87.36"))
DOUBLED = (date(2026, 9, 22), "Noodle Bar", Decimal("41.80"))
SWAPPED = (date(2026, 10, 27), "Bookshop", Decimal("45.10"), Decimal("54.10"))


def main():
    txns, seen = [], set()
    while len(txns) < 1400:
        d = date(2026, 9, 1) + timedelta(days=random.randrange(61))
        payee = random.choice(list(PAYEES))
        amount = Decimal(random.randint(300, 18000)) / 100
        key = (d, amount)
        # Amounts are unique per day, so a match on (day, amount) is unambiguous,
        # except for the coffee decoy below, which is the same in both files.
        if key in seen or amount in (MISSING[2], DOUBLED[2], SWAPPED[2], SWAPPED[3], Decimal("4.50"), Decimal("19.99")):
            continue
        seen.add(key)
        txns.append([d, payee, amount, f"{random.randint(100, 999)}"])
    for d, payee, amount in [MISSING[:3], DOUBLED[:3], SWAPPED[:3]]:
        txns.append([d, payee, amount, "412"])
    txns += [[date(2026, 10, 3), "The Daily Grind", Decimal("4.50"), "101"],
             [date(2026, 10, 3), "The Daily Grind", Decimal("4.50"), "101"]]
    refund = [date(2026, 10, 11), "Hardware Hub", Decimal("-19.99"), "230"]
    txns.append(refund)
    txns.sort(key=lambda t: (t[0], t[1], t[2]))

    bank = io.StringIO()
    w = csv.writer(bank, lineterminator="\n")
    w.writerow(["Posting Date", "Description", "Amount"])
    for d, payee, amount, store in txns:
        w.writerow([d.isoformat(), f"{PAYEES[payee]} #{store}", f"{-amount:.2f}"])

    budget = io.StringIO()
    w = csv.writer(budget, lineterminator="\n")
    w.writerow(["Date", "Payee", "Category", "Outflow", "Inflow"])
    for d, payee, amount, _ in txns:
        if (d, payee, amount) == MISSING:
            continue
        rows = 2 if (d, payee, amount) == DOUBLED else 1
        if (d, payee, amount) == SWAPPED[:3]:
            amount = SWAPPED[3]
        out, inflow = (f"{amount:.2f}", "") if amount > 0 else ("", f"{-amount:.2f}")
        for _ in range(rows):
            w.writerow([d.strftime("%d %b %Y"), payee, "Spending", out, inflow])

    root = pathlib.Path("fixture")
    root.mkdir(exist_ok=True)
    (root / "bank.csv").write_text(bank.getvalue())
    (root / "budget.csv").write_text(budget.getvalue())
    (root / "README.md").write_text(
        "Two exports of the same card account, September and October 2026.\n\n"
        "- `bank.csv`: the bank's export. Charges are negative, refunds positive.\n"
        "- `budget.csv`: our budget app's export. Charges are outflows, refunds inflows.\n"
        "  Payees are cleaned up there, so they don't read like the bank's.\n"
    )


if __name__ == "__main__":
    main()
