import re


def is_email(text: str) -> bool:
    """A name, an @, and a domain with at least one dot."""
    return re.fullmatch(r"[^@\s]+@[^@\s.]+(\.[^@\s.]+)+", text) is not None


def is_zip(text: str) -> bool:
    """A US ZIP code: 12345 or 12345-6789."""
    return re.fullmatch(r"\d{5}(-\d{4})?", text) is not None


def clamp(value: float, low: float, high: float) -> float:
    """`value` kept within [low, high]."""
    return max(low, min(high, value))
