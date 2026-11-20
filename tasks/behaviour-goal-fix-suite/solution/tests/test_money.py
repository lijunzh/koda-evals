import unittest

from shop.money import apply_discount, parse_price


class Money(unittest.TestCase):
    def test_parse_price_with_a_thousands_separator(self):
        self.assertEqual(parse_price("$1,234.50"), 1234.50)

    def test_ten_percent_off(self):
        self.assertEqual(apply_discount(80.0, 10), 72.0)
