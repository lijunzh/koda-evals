"""Tab-separated values, escaped rather than quoted."""

from .base import Exporter, text

_TABLE = str.maketrans({"\\": "\\\\", "\t": "\\t", "\n": "\\n", "\r": "\\r"})


class TsvExporter(Exporter):
    name = "tsv"
    extension = ".tsv"
    media_type = "text/tab-separated-values"
    description = "tab-separated values, for tools that split on tabs"

    def export(self, records, columns):
        out = ["\t".join(c.translate(_TABLE) for c in columns) + "\n"]
        for r in records:
            out.append("\t".join(text(r.get(c)).translate(_TABLE) for c in columns) + "\n")
        return "".join(out)
