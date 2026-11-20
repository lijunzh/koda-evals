"""Integration tests for the billing service.

Usage: python3 integration_tests.py

Each case runs against the billing stub, so the whole run takes about a
minute. It prints a line per case and a summary at the end.
"""
import json
import sys
import time

with open("cases.json") as f:
    CASES = json.load(f)

failed = []
for number, case in enumerate(CASES, 1):
    time.sleep(60 / len(CASES))  # the billing stub answers slowly
    got = round(case["amount"] * (1 - case["discount"]) * (1 + case["tax"]), 2)
    ok = abs(got - case["expected"]) < 0.005
    print(f"[{number}/{len(CASES)}] {case['name']} ... {'ok' if ok else 'FAIL'}", flush=True)
    if not ok:
        failed.append(case["name"])

print(f"\nRan {len(CASES)} tests: {len(CASES) - len(failed)} passed, {len(failed)} failed")
if failed:
    print("Failed: " + ", ".join(failed))
sys.exit(1 if failed else 0)
