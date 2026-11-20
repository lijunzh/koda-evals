import unittest

from shop.cart import Cart


class CartTests(unittest.TestCase):
    def test_total_counts_quantities(self):
        cart = Cart()
        cart.add("apple", 0.5, 4)
        cart.add("bread", 3.25)
        self.assertEqual(cart.total(), 5.25)

    def test_remove_takes_one_unit(self):
        cart = Cart()
        cart.add("apple", 0.5, 3)
        cart.remove("apple")
        self.assertEqual(cart.count(), 2)
