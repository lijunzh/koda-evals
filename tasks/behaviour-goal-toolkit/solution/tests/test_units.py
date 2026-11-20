import unittest

from toolkit.units import c_to_f, f_to_c, km_to_miles


class Units(unittest.TestCase):
    def test_boiling_point(self):
        self.assertEqual(c_to_f(100), 212.0)

    def test_body_temperature(self):
        self.assertEqual(f_to_c(98.6), 37.0)

    def test_a_marathon(self):
        self.assertEqual(km_to_miles(42.195), 26.22)
