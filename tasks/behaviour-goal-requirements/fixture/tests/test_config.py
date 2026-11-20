import os
import tempfile
import unittest

from app.config import load


class Config(unittest.TestCase):
    def test_defaults_fill_the_gaps(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "config.toml")
            with open(path, "w") as f:
                f.write('log_level = "debug"\n')
            settings = load(path)
            self.assertEqual(settings["port"], 8080)
            self.assertEqual(settings["log_level"], "debug")
