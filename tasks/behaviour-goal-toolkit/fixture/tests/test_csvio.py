import unittest

from toolkit.csvio import parse_int_field, parse_row, to_row


class Csv(unittest.TestCase):
    def test_quoted_comma(self):
        self.assertEqual(parse_row('1,"Smith, Jane",NY'), ["1", "Smith, Jane", "NY"])

    def test_quotes_a_comma(self):
        self.assertEqual(to_row(["1", "Smith, Jane"]), '1,"Smith, Jane"')

    def test_empty_int_field(self):
        self.assertIsNone(parse_int_field("  "))
