"""Orders read from a CSV file."""
import csv


def line_total_cents(quantity: int, unit_cents: int, discount_pct: int) -> int:
    """What a line costs in cents: quantity times unit price, less a
    percentage discount, rounded to the nearest cent (halves round up)."""
    gross = quantity * unit_cents
    return gross - gross * discount_pct // 100


def read_orders(path: str) -> list[dict]:
    """The orders in `path`, one per order id, each with its customer and
    its total in cents."""
    orders: dict[str, dict] = {}
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            order = orders.setdefault(row["order_id"], {"id": row["order_id"], "customer": row["customer"], "total_cents": 0})
            order["total_cents"] += line_total_cents(int(row["quantity"]), int(row["unit_cents"]), int(row["discount_pct"]))
    return list(orders.values())
