import math


def mean(values: list[float]) -> float:
    """The average, or 0.0 for no values."""
    return sum(values) / len(values) if values else 0.0


def median(values: list[float]) -> float:
    """The middle value; for an even count, the average of the two middle ones."""
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / 2


def percentile(values: list[float], p: float) -> float:
    """The nearest-rank percentile: the smallest value with at least p% at or below it."""
    ordered = sorted(values)
    rank = max(1, math.ceil(p / 100 * len(ordered)))
    return ordered[rank - 1]
