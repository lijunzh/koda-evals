"""Hidden check: pipeline.py, the unit tests and the data are unchanged, and
the three fixes the pipeline reveals one stage at a time hold, on the data
and on more cases: line totals round to the nearest cent (halves up), the
report lists customers largest total first (ties by name), and the export
gives integer cents under exactly the keys the validator asks for. These are
the pipeline's own stage checks, without its waits."""

import csv
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest

from shop.export import build_export
from shop.orders import line_total_cents
from shop.report import build_report

UNCHANGED = {
    "pipeline.py": "06f99c11f5f429bef77f68f1504962559be6ae0f5a3984869ff963008bad6716",
    "tests/test_orders.py": "727b986e0dc76cc8400bec97dad000a627e251e0573a3f2a3caac2bb58a756e3",
    "data/orders.csv": "68b4b6f67621a3442832b9fb59b76ae6b1f5324bc9f06817b05e3f5e07d1640f",
}


def write_csv(rows):
    f = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, newline="")
    w = csv.writer(f)
    w.writerow(["order_id", "customer", "quantity", "unit_cents", "discount_pct"])
    w.writerows(rows)
    f.close()
    return f.name


def cents(text):
    dollars, rest = text.lstrip("$").split(".")
    return int(dollars) * 100 + int(rest)


class Hidden(unittest.TestCase):
    def test_files_unchanged(self):
        for path, digest in UNCHANGED.items():
            with open(path, "rb") as f:
                self.assertEqual(hashlib.sha256(f.read()).hexdigest(), digest, path)

    def test_unit_stage(self):
        run = subprocess.run(
            [sys.executable, "-m", "unittest", "tests.test_orders"], capture_output=True, text=True
        )
        self.assertEqual(run.returncode, 0, run.stderr[-1500:])

    def test_line_totals_round_to_the_nearest_cent(self):
        for q, unit, pct, want in [
            (1, 107, 10, 96), (1, 105, 10, 95), (3, 333, 7, 929), (2, 999, 15, 1698),
            (7, 1, 50, 4), (1, 1, 50, 1), (4, 250, 0, 1000), (5, 1999, 33, 6697),
        ]:
            self.assertEqual(line_total_cents(q, unit, pct), want, (q, unit, pct))

    def test_report_stage(self):
        lines = build_report("data/orders.csv").splitlines()
        rows = [(line.split()[0], cents(line.split()[1])) for line in lines[:-1]]
        self.assertEqual(rows, [("bo", 6470), ("cy", 6058), ("ana", 2859), ("dee", 950)])
        self.assertEqual(lines[-1].split(), ["TOTAL", "$163.37"])

    def test_report_ties_by_name(self):
        path = write_csv([["1", "zed", 1, 500, 0], ["2", "amy", 1, 500, 0], ["3", "max", 2, 999, 0]])
        try:
            names = [line.split()[0] for line in build_report(path).splitlines()]
        finally:
            os.unlink(path)
        self.assertEqual(names, ["max", "amy", "zed", "TOTAL"])

    def test_export_stage(self):
        data = json.loads(build_export("data/orders.csv"))
        self.assertEqual(set(data), {"orders", "grand_total_cents"})
        for order in data["orders"]:
            self.assertEqual(set(order), {"id", "customer", "total_cents"})
            self.assertIs(type(order["total_cents"]), int)
        self.assertEqual(data["grand_total_cents"], 16337)

    def test_export_in_integer_cents(self):
        path = write_csv([["9", "kim", 3, 333, 7], ["9", "kim", 1, 100, 0], ["8", "lu", 1, 1005, 10]])
        try:
            data = json.loads(build_export(path))
        finally:
            os.unlink(path)
        by_id = {o["id"]: o for o in data["orders"]}
        self.assertEqual(by_id["9"], {"id": "9", "customer": "kim", "total_cents": 1029})
        self.assertEqual(by_id["8"], {"id": "8", "customer": "lu", "total_cents": 905})
        self.assertIs(type(data["grand_total_cents"]), int)
        self.assertEqual(data["grand_total_cents"], 1934)


if __name__ == "__main__":
    unittest.main()
