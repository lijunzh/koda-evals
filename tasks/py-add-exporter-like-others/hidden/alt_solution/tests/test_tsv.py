import unittest

from reportkit.exporters import get_exporter


class TestTsv(unittest.TestCase):
    def test_backslash_is_escaped_first(self):
        self.assertEqual(get_exporter("tsv").export([{"a": "\\t"}], ["a"]), "a\n\\\\t\n")
