"""Every format, by name."""

from typing import Dict, Type

from .base import Exporter
from .csv_exporter import CsvExporter
from .json_exporter import JsonExporter

EXPORTERS: Dict[str, Type[Exporter]] = {
    "csv": CsvExporter,
    "json": JsonExporter,
}


def get_exporter(name: str) -> Exporter:
    try:
        return EXPORTERS[name]()
    except KeyError:
        raise ValueError(f"unknown format {name!r}; known: {', '.join(EXPORTERS)}") from None
