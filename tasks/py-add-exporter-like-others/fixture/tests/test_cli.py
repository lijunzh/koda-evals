import contextlib
import io
import os
import tempfile
import unittest

from reportkit.cli import main


class CliTest(unittest.TestCase):
    def test_list_formats(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(main(["--list-formats"]), 0)
        self.assertIn("csv", out.getvalue())
        self.assertIn("json", out.getvalue())

    def test_export(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "v.csv")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(["--format", "csv", "examples/visits.json", path]), 0)
            with open(path, encoding="utf-8") as f:
                self.assertTrue(f.readline().startswith("day,clinic,visits,note"))
