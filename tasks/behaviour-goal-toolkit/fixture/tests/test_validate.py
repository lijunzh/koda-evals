import unittest

from toolkit.validate import clamp, is_email, is_zip


class Validate(unittest.TestCase):
    def test_domain_needs_a_dot(self):
        self.assertFalse(is_email("someone@localhost"))

    def test_zip_plus_four(self):
        self.assertTrue(is_zip("12345-6789"))

    def test_clamp(self):
        self.assertEqual(clamp(5, 0, 10), 5)
