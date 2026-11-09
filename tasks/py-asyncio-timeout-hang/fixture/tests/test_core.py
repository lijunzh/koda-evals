import asyncio
import unittest

from fetchall import fetch_all


class FetchAllTest(unittest.TestCase):
    def test_every_url_answers(self):
        async def fetch(url):
            await asyncio.sleep(0.01)
            return url.encode()

        got = asyncio.run(fetch_all(["a", "b", "c"], fetch, timeout=1))
        self.assertEqual(got, {"a": b"a", "b": b"b", "c": b"c"})

    def test_a_failure_is_retried(self):
        calls = []

        async def fetch(url):
            calls.append(url)
            if len(calls) < 3:
                raise ConnectionError("reset")
            return b"ok"

        got = asyncio.run(fetch_all(["a"], fetch, timeout=1, backoff=0))
        self.assertEqual(got, {"a": b"ok"})
        self.assertEqual(len(calls), 3)

    def test_duplicates_are_fetched_once(self):
        calls = []

        async def fetch(url):
            calls.append(url)
            return b"x"

        got = asyncio.run(fetch_all(["a", "a"], fetch, timeout=1))
        self.assertEqual(got, {"a": b"x"})
        self.assertEqual(calls, ["a"])


if __name__ == "__main__":
    unittest.main()
