"""The JSON export of every order, for the accounting system."""
import json

from shop.orders import read_orders


def build_export(path: str) -> str:
    """Every order with its total, and the grand total, as JSON."""
    orders = [
        {"id": o["id"], "customer": o["customer"], "total": o["total_cents"] / 100}
        for o in read_orders(path)
    ]
    return json.dumps({"orders": orders, "grand_total": sum(o["total"] for o in orders)}, indent=2)
