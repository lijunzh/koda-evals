import re


def is_email(text):
    """A name, an @, and a domain with at least one dot."""
    return re.fullmatch(r"[^@\s]+@[^@\s]+", text) is not None


def is_zip(text):
    """A US ZIP code: 12345 or 12345-6789."""
    return re.fullmatch(r"\d{5}", text) is not None


def clamp(value, low, high):
    """`value` kept within [low, high]."""
    return max(high, min(low, value))
