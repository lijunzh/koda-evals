"""ledger: python3 ledger.py export IN.json OUT.csv | python3 ledger.py stats REPORT.csv"""

import json
import sys
from datetime import date
from decimal import Decimal

from ledger.export import write_report
from ledger.model import Transaction
from ledger.summary import count_rows


def load_json(path):
    with open(path, encoding="utf-8") as f:
        rows = json.load(f)
    return [Transaction(date.fromisoformat(r["date"]), r["payee"], r.get("memo", ""),
                        Decimal(str(r["amount"]))) for r in rows]


def main(argv):
    if len(argv) == 3 and argv[0] == "export":
        n = write_report(argv[2], load_json(argv[1]))
        print(f"wrote {n} transactions to {argv[2]}")
        return 0
    if len(argv) == 2 and argv[0] == "stats":
        print(f"{count_rows(argv[1])} transactions")
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
