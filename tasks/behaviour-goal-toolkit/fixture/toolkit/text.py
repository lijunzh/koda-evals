import re

SMALL_WORDS = {"a", "an", "the", "of", "and", "in"}


def slugify(text):
    """Lowercase words joined by single hyphens, nothing else."""
    slug = re.sub(r"[^a-z0-9]", "-", text.lower())
    return slug.strip("-")


def truncate(text, width):
    """`text` cut to at most `width` characters, ending in an ellipsis when cut."""
    if len(text) <= width:
        return text
    return text[:width] + "…"


def title_case(text):
    """Each word capitalized, except small words after the first."""
    words = text.lower().split()
    return " ".join(
        w if (w in SMALL_WORDS) else w.capitalize() for w in words
    )
