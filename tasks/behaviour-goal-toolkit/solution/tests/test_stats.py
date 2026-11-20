import unittest

from toolkit.stats import mean, median, percentile


class Stats(unittest.TestCase):
    def test_mean_of_nothing(self):
        self.assertEqual(mean([]), 0.0)

    def test_median_of_an_even_count(self):
        self.assertEqual(median([4, 1, 3, 2]), 2.5)

    def test_percentile_nearest_rank(self):
        self.assertEqual(percentile([15, 20, 35, 40, 50], 40), 20)
