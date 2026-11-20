"""Prices as people type them, and discounts."""


def parse_price(text):
    """A price such as "$1,234.50" or "12" as a float."""
    return float(text.strip().lstrip("$").replace(",", ""))


def apply_discount(price, percent):
    """`price` with `percent` off, to the cent."""
    return round(price * (1 - percent / 100), 2)
