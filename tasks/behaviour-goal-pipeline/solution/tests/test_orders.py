import unittest

from shop.orders import line_total_cents


class LineTotal(unittest.TestCase):
    def test_no_discount(self):
        self.assertEqual(line_total_cents(3, 250, 0), 750)

    def test_a_discount_that_divides_evenly(self):
        self.assertEqual(line_total_cents(2, 500, 10), 900)

    def test_half_a_cent_rounds_up(self):
        # 1 x 105 cents less 10% is 94.5 cents: 95.
        self.assertEqual(line_total_cents(1, 105, 10), 95)

    def test_less_than_half_a_cent_rounds_down(self):
        # 1 x 107 cents less 10% is 96.3 cents: 96.
        self.assertEqual(line_total_cents(1, 107, 10), 96)
