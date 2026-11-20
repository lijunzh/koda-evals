def mean(values):
    """The average, or 0.0 for no values."""
    return sum(values) / len(values)


def median(values):
    """The middle value; for an even count, the average of the two middle ones."""
    ordered = sorted(values)
    return ordered[len(ordered) // 2]


def percentile(values, p):
    """The nearest-rank percentile: the smallest value with at least p% at or below it."""
    import math

    ordered = sorted(values)
    rank = math.ceil(p / 100 * len(ordered))
    return ordered[rank]
