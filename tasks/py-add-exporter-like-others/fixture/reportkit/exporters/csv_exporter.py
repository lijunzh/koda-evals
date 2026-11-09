"""CSV."""

import csv
import io

from .base import Exporter, text


class CsvExporter(Exporter):
    name = "csv"
    extension = ".csv"
    media_type = "text/csv"

    def export(self, records, columns):
        buf = io.StringIO()
        w = csv.writer(buf, lineterminator="\n")
        w.writerow(columns)
        for r in records:
            w.writerow([text(r.get(c)) for c in columns])
        return buf.getvalue()
