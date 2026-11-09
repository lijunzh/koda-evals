"""TSV."""

from .base import Exporter, text

# TSV can't quote, so it escapes (CONTRIBUTING.md). Backslash first, so the
# backslashes the others add aren't escaped again.
_ESCAPES = [("\\", "\\\\"), ("\t", "\\t"), ("\n", "\\n"), ("\r", "\\r")]


def escape(value: str) -> str:
    for raw, escaped in _ESCAPES:
        value = value.replace(raw, escaped)
    return value


class TsvExporter(Exporter):
    name = "tsv"
    extension = ".tsv"
    media_type = "text/tab-separated-values"

    def export(self, records, columns):
        lines = ["\t".join(escape(c) for c in columns)]
        for r in records:
            lines.append("\t".join(escape(text(r.get(c))) for c in columns))
        return "".join(line + "\n" for line in lines)
