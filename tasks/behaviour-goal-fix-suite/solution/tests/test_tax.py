import unittest

from shop.tax import rate_for, with_tax


class Tax(unittest.TestCase):
    def test_lowercase_state(self):
        self.assertEqual(rate_for("tx"), 0.0625)

    def test_rounds_half_up_to_the_cent(self):
        self.assertEqual(with_tax(10.00, "CA"), 10.73)
