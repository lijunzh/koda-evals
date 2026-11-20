import unittest

from app import clock
from app.api import App


class RateLimit(unittest.TestCase):
    def setUp(self):
        self.real = clock.now
        self.t = 0.0
        clock.now = lambda: self.t

    def tearDown(self):
        clock.now = self.real

    def test_over_the_limit_gets_429_until_the_minute_passes(self):
        app = App({"rate_limit_per_minute": 2})
        ask = lambda: app.handle({"user": "ann", "path": "/health"})["status"]
        self.assertEqual([ask(), ask(), ask()], [200, 200, 429])
        self.t = 61
        self.assertEqual(ask(), 200)
