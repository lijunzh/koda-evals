import csv
import io
import unittest
from datetime import date

from reportkit.exporters import EXPORTERS, get_exporter


class CsvTest(unittest.TestCase):
    def test_registered(self):
        self.assertEqual(EXPORTERS["csv"].extension, ".csv")
        self.assertEqual(EXPORTERS["csv"].media_type, "text/csv")

    def test_round_trips_through_a_csv_reader(self):
        out = get_exporter("csv").export(
            [{"day": date(2026, 11, 3), "note": "a, b\nc", "n": None}], ["day", "note", "n"])
        self.assertEqual(list(csv.reader(io.StringIO(out))),
                         [["day", "note", "n"], ["2026-11-03", "a, b\nc", ""]])


if __name__ == "__main__":
    unittest.main()
