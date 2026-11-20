import re

SMALL_WORDS = {"a", "an", "the", "of", "and", "in"}


def slugify(text: str) -> str:
    """Lowercase words joined by single hyphens, nothing else."""
    slug = re.sub(r"[^a-z0-9]+", "-", text.lower())
    return slug.strip("-")


def truncate(text: str, width: int) -> str:
    """`text` cut to at most `width` characters, ending in an ellipsis when cut."""
    if len(text) <= width:
        return text
    return text[: width - 1] + "…"


def title_case(text: str) -> str:
    """Each word capitalized, except small words after the first."""
    words = text.lower().split()
    return " ".join(
        w if (i > 0 and w in SMALL_WORDS) else w.capitalize() for i, w in enumerate(words)
    )
