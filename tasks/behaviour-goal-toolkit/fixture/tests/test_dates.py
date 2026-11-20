import unittest

from toolkit.dates import add_days, days_in_month, is_leap


class Dates(unittest.TestCase):
    def test_century_years(self):
        self.assertFalse(is_leap(1900))

    def test_february_in_a_leap_year(self):
        self.assertEqual(days_in_month(2024, 2), 29)

    def test_add_days(self):
        self.assertEqual(add_days("2026-01-30", 3), "2026-02-02")
