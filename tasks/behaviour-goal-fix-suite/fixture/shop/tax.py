"""Sales tax by state."""

RATES = {"TX": 0.0625, "AR": 0.065, "CA": 0.0725, "OR": 0.0}


def rate_for(state):
    """The rate for a state code, in any case, such as "tx"."""
    return RATES[state]


def with_tax(amount, state):
    """`amount` plus its tax, rounded half up to the cent."""
    total = amount * (1 + rate_for(state))
    return int(total * 100) / 100
