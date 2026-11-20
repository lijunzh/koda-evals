import unittest

from toolkit.text import slugify, title_case, truncate


class Text(unittest.TestCase):
    def test_slugify_collapses_separators(self):
        self.assertEqual(slugify("Hello,  World!"), "hello-world")

    def test_truncate_fits_the_width(self):
        self.assertEqual(truncate("abcdefgh", 5), "abcd…")

    def test_title_case_capitalizes_a_small_first_word(self):
        self.assertEqual(title_case("the lord of the rings"), "The Lord of the Rings")
