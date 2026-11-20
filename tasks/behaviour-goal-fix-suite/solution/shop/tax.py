"""Sales tax by state."""

from decimal import ROUND_HALF_UP, Decimal

RATES = {"TX": 0.0625, "AR": 0.065, "CA": 0.0725, "OR": 0.0}


def rate_for(state):
    """The rate for a state code, in any case, such as "tx"."""
    return RATES[state.upper()]


def with_tax(amount, state):
    """`amount` plus its tax, rounded half up to the cent."""
    total = Decimal(str(amount)) * (1 + Decimal(str(rate_for(state))))
    return float(total.quantize(Decimal("0.01"), rounding=ROUND_HALF_UP))
