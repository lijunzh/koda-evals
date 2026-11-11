"""Reconcile fixture/bank.csv against fixture/budget.csv independently of
make_data.py, and print every difference. Run from the task directory."""

import csv
from collections import Counter
from datetime import datetime
from decimal import Decimal

with open("fixture/bank.csv", newline="") as f:
    bank = Counter((r["Posting Date"], -Decimal(r["Amount"])) for r in csv.DictReader(f))
with open("fixture/budget.csv", newline="") as f:
    budget = Counter(
        (datetime.strptime(r["Date"], "%d %b %Y").date().isoformat(),
         Decimal(r["Outflow"]) if r["Outflow"] else -Decimal(r["Inflow"]))
        for r in csv.DictReader(f))
print("in the bank, not (or less often) in the budget:", sorted((bank - budget).items()))
print("in the budget, not (or less often) in the bank:", sorted((budget - bank).items()))
print("rows:", sum(bank.values()), "bank,", sum(budget.values()), "budget")
