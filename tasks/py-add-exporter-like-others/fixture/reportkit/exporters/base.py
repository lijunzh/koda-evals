"""What every exporter shares."""

from datetime import date, datetime
from decimal import Decimal
from typing import Any, Dict, Iterable, List

Record = Dict[str, Any]


class Exporter:
    """Turns records into the text of one file."""

    name: str = ""
    extension: str = ""
    media_type: str = ""

    def export(self, records: Iterable[Record], columns: List[str]) -> str:
        raise NotImplementedError


def text(value: Any) -> str:
    """A value as text formats write it: None empty, dates ISO, numbers as they are."""
    if value is None:
        return ""
    if isinstance(value, (date, datetime)):
        return value.isoformat()
    if isinstance(value, Decimal):
        return format(value, "f")
    return str(value)
