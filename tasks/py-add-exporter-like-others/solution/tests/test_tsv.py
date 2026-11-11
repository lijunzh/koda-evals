import unittest
from datetime import date

from reportkit.exporters import EXPORTERS, get_exporter


class TsvTest(unittest.TestCase):
    def test_registered(self):
        self.assertEqual(EXPORTERS["tsv"].extension, ".tsv")
        self.assertEqual(EXPORTERS["tsv"].media_type, "text/tab-separated-values")

    def test_escapes_instead_of_quoting(self):
        out = get_exporter("tsv").export(
            [{"day": date(2026, 11, 3), "note": "a\tb\nc\\d", "n": None}], ["day", "note", "n"])
        self.assertEqual(out, "day\tnote\tn\n2026-11-03\ta\\tb\\nc\\\\d\t\n")


if __name__ == "__main__":
    unittest.main()
