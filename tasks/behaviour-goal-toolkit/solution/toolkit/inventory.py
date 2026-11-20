class Stock:
    """Items on hand: name -> (quantity, unit price)."""

    def __init__(self) -> None:
        self.items: dict[str, tuple[int, float]] = {}

    def add(self, name: str, quantity: int, price: float) -> None:
        """Add `quantity` of `name` at `price` each."""
        held, _ = self.items.get(name, (0, price))
        self.items[name] = (held + quantity, price)

    def remove(self, name: str, quantity: int) -> None:
        """Take `quantity` of `name` out; ValueError if there isn't that much."""
        held, price = self.items.get(name, (0, 0.0))
        if quantity > held:
            raise ValueError(f"only {held} of {name} on hand")
        self.items[name] = (held - quantity, price)

    def low_stock(self, threshold: int) -> list[str]:
        """Names with fewer than `threshold` on hand, sorted."""
        return sorted(n for n, (q, _) in self.items.items() if q < threshold)

    def total_value(self) -> float:
        """What everything on hand is worth, to two decimals."""
        return round(sum(q * price for q, price in self.items.values()), 2)
