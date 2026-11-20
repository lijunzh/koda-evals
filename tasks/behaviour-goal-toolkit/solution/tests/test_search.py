import unittest

from toolkit.search import binary_search, fuzzy_contains, top_k


class Search(unittest.TestCase):
    def test_finds_the_last_item(self):
        self.assertEqual(binary_search([1, 3, 5, 7], 7), 3)

    def test_ignores_case(self):
        self.assertTrue(fuzzy_contains("Hello World", "WORLD"))

    def test_highest_first(self):
        self.assertEqual(top_k({"a": 1, "b": 5, "c": 3}, 2), ["b", "c"])
