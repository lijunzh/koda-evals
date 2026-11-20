import csv
import io


def parse_row(line: str) -> list[str]:
    """Fields of one CSV line; a quoted field may hold commas."""
    return next(csv.reader([line]))


def to_row(fields: list[str]) -> str:
    """One CSV line; a field holding a comma or a quote is quoted, quotes doubled."""
    out = io.StringIO()
    csv.writer(out, lineterminator="").writerow(fields)
    return out.getvalue()


def parse_int_field(text: str) -> int | None:
    """An integer field, ignoring spaces around it; None when empty."""
    text = text.strip()
    return int(text) if text else None
