"""A shopping cart."""


class Cart:
    def __init__(self):
        self.lines = {}  # sku -> [price, quantity]

    def add(self, sku, price, quantity=1):
        line = self.lines.setdefault(sku, [price, 0])
        line[1] += quantity

    def remove(self, sku):
        """Take one unit of `sku` out; the line goes when none are left."""
        line = self.lines[sku]
        line[1] -= 1
        if line[1] == 0:
            del self.lines[sku]

    def total(self):
        """What the cart costs, to the cent."""
        return round(sum(price * quantity for price, quantity in self.lines.values()), 2)

    def count(self):
        """How many units are in the cart."""
        return sum(quantity for _, quantity in self.lines.values())
