"""JSON."""

import json
from datetime import date, datetime
from decimal import Decimal

from .base import Exporter


def _plain(value):
    if isinstance(value, (date, datetime)):
        return value.isoformat()
    if isinstance(value, Decimal):
        return format(value, "f")
    return value


class JsonExporter(Exporter):
    name = "json"
    extension = ".json"
    media_type = "application/json"

    def export(self, records, columns):
        rows = [{c: _plain(r.get(c)) for c in columns} for r in records]
        return json.dumps(rows, indent=2, ensure_ascii=False) + "\n"
