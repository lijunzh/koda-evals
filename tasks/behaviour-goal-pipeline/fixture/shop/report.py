"""The sales report: one line per customer, the biggest spender first."""
from shop.orders import read_orders


def dollars(cents: int) -> str:
    return f"${cents // 100}.{cents % 100:02d}"


def build_report(path: str) -> str:
    """Each customer's total across their orders, largest first (ties by
    name), then the grand total."""
    totals: dict[str, int] = {}
    for order in read_orders(path):
        totals[order["customer"]] = totals.get(order["customer"], 0) + order["total_cents"]
    lines = [f"{name:<10}{dollars(cents):>12}" for name, cents in sorted(totals.items())]
    lines.append(f"{'TOTAL':<10}{dollars(sum(totals.values())):>12}")
    return "\n".join(lines) + "\n"
