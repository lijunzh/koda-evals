import unittest

from toolkit.inventory import Stock


class Inventory(unittest.TestCase):
    def test_remove_more_than_held(self):
        stock = Stock()
        stock.add("bolt", 3, 0.25)
        with self.assertRaises(ValueError):
            stock.remove("bolt", 5)

    def test_low_stock_is_strictly_below(self):
        stock = Stock()
        stock.add("bolt", 3, 0.25)
        stock.add("nut", 10, 0.1)
        self.assertEqual(stock.low_stock(3), [])

    def test_total_value_counts_quantities(self):
        stock = Stock()
        stock.add("bolt", 4, 0.25)
        stock.add("nut", 10, 0.1)
        self.assertEqual(stock.total_value(), 2.0)
