import unittest

from app.api import App


class Api(unittest.TestCase):
    def setUp(self):
        self.app = App({"port": 8080, "log_level": "info"})

    def test_health(self):
        self.assertEqual(self.app.handle({"user": "ann", "path": "/health"})["status"], 200)

    def test_a_note(self):
        answer = self.app.handle({"user": "ann", "path": "/notes/welcome"})
        self.assertEqual(answer, {"status": 200, "body": "Hello from the notes service."})

    def test_a_missing_note(self):
        self.assertEqual(self.app.handle({"user": "ann", "path": "/notes/x"})["status"], 404)
