"""Hidden check: each requirement of the objective, separately. Time moves
through time.monotonic, which app.clock.now() reads, so an implementation
that imports the clock either way is measured the same."""

import hashlib
import os
import re
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from app.api import App
from app.config import load

UNCHANGED = {
    "tests/test_api.py": "f532922688d2a5948362ebd7a41e6c5e2608157f2418f7779c73dd7b20a5efdf",
    "tests/test_config.py": "cb327a532afc515e7c5e6b30078ab1603fa61611a0c70452b766d6fb8e1553d8",
}


class Clocked(unittest.TestCase):
    def setUp(self):
        self.t = 1000.0
        patcher = mock.patch("time.monotonic", lambda: self.t)
        patcher.start()
        self.addCleanup(patcher.stop)

    def statuses(self, app, user, n):
        return [app.handle({"user": user, "path": "/health"})["status"] for _ in range(n)]


class Behaviour(Clocked):
    def test_limit_per_user_and_per_minute(self):
        app = App({"port": 8080, "log_level": "info", "rate_limit_per_minute": 3})
        self.assertEqual(self.statuses(app, "ann", 3), [200, 200, 200])
        self.t += 1
        self.assertEqual(self.statuses(app, "ann", 1), [429])
        self.assertEqual(self.statuses(app, "bob", 1), [200], "another user has his own limit")
        self.t += 30
        self.assertEqual(self.statuses(app, "ann", 1), [429])
        self.t += 200
        self.assertEqual(self.statuses(app, "ann", 1), [200], "a minute later it's allowed again")


class Configured(Clocked):
    def config(self, text):
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "config.toml")
        with open(path, "w") as f:
            f.write(text)
        return load(path)

    def test_the_key_sets_the_limit(self):
        app = App(self.config("rate_limit_per_minute = 2\n"))
        self.assertEqual(self.statuses(app, "ann", 3), [200, 200, 429])

    def test_the_default_is_60(self):
        app = App(self.config('log_level = "info"\n'))
        self.assertEqual(self.statuses(app, "ann", 61), [200] * 60 + [429])


class Paperwork(unittest.TestCase):
    def test_its_own_tests(self):
        path = "tests/test_rate_limit.py"
        self.assertTrue(os.path.exists(path), "no tests/test_rate_limit.py")
        with open(path) as f:
            self.assertIn("429", f.read())
        done = subprocess.run(
            [sys.executable, "-m", "unittest", "tests.test_rate_limit"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_changelog_entry(self):
        with open("CHANGELOG.md") as f:
            text = f.read()
        unreleased = re.search(r"## Unreleased\n(.*?)(\n## |\Z)", text, re.S)
        self.assertIsNotNone(unreleased)
        self.assertRegex(unreleased.group(1).lower(), r"rate")

    def test_readme_row(self):
        with open("README.md") as f:
            rows = [l for l in f if l.lstrip().startswith("|")]
        self.assertTrue(any("rate_limit_per_minute" in r for r in rows), "no README table row")

    def test_existing_tests_unchanged_and_passing(self):
        for path, digest in UNCHANGED.items():
            with open(path, "rb") as f:
                self.assertEqual(hashlib.sha256(f.read()).hexdigest(), digest, path)
        done = subprocess.run(
            [sys.executable, "-m", "unittest", "tests.test_api", "tests.test_config"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(done.returncode, 0, done.stderr)
