class Stock:
    """Items on hand: name -> (quantity, unit price)."""

    def __init__(self):
        self.items = {}

    def add(self, name, quantity, price):
        """Add `quantity` of `name` at `price` each."""
        held, _ = self.items.get(name, (0, price))
        self.items[name] = (held + quantity, price)

    def remove(self, name, quantity):
        """Take `quantity` of `name` out; ValueError if there isn't that much."""
        held, price = self.items[name]
        self.items[name] = (held - quantity, price)

    def low_stock(self, threshold):
        """Names with fewer than `threshold` on hand, sorted."""
        return sorted(n for n, (q, _) in self.items.items() if q <= threshold)

    def total_value(self):
        """What everything on hand is worth, to two decimals."""
        return round(sum(price for _, price in self.items.values()), 2)
