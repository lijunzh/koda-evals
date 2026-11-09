"""Hidden tests: the timeout returns, keeps what finished, and leaves nothing running."""

import asyncio
import threading
import time
import unittest

from fetchall import fetch_all


def run_bounded(make_coro, limit=5.0):
    """Run `make_coro()` in its own thread and event loop; fail if it hangs."""
    box = {}

    def target():
        try:
            box["value"] = asyncio.run(make_coro())
        except BaseException as e:  # reported below
            box["error"] = e

    thread = threading.Thread(target=target, daemon=True)
    thread.start()
    thread.join(limit)
    if thread.is_alive():
        raise AssertionError(f"still running after {limit}s: it hangs")
    if "error" in box:
        raise box["error"]
    return box["value"]


class EvalHiddenTimeout(unittest.TestCase):
    def test_a_host_that_never_answers_times_out(self):
        async def fetch(url):
            if url == "slow":
                await asyncio.sleep(3600)
            return url.encode()

        start = time.monotonic()
        got = run_bounded(lambda: fetch_all(["fast", "slow"], fetch, timeout=0.3))
        self.assertLess(time.monotonic() - start, 2.5)
        self.assertEqual(got, {"fast": b"fast", "slow": None})

    def test_what_finished_is_kept_in_order(self):
        async def fetch(url):
            if url.startswith("slow"):
                await asyncio.sleep(3600)
            await asyncio.sleep(0.01)
            return url.encode()

        got = run_bounded(lambda: fetch_all(["a", "slow1", "b", "slow2"], fetch, timeout=0.3))
        self.assertEqual(list(got), ["a", "slow1", "b", "slow2"])
        self.assertEqual(got, {"a": b"a", "slow1": None, "b": b"b", "slow2": None})

    def test_the_slow_fetch_is_cancelled_and_nothing_is_left_running(self):
        cancelled = []

        async def fetch(url):
            try:
                await asyncio.sleep(3600)
            except asyncio.CancelledError:
                cancelled.append(url)
                raise
            return b""

        async def scenario():
            got = await fetch_all(["slow"], fetch, timeout=0.2)
            await asyncio.sleep(0.05)
            others = [t for t in asyncio.all_tasks() if t is not asyncio.current_task()]
            return got, [t for t in others if not t.done()]

        got, running = run_bounded(scenario)
        self.assertEqual(got, {"slow": None})
        self.assertEqual(cancelled, ["slow"])
        self.assertEqual(running, [])

    def test_the_slow_fetch_is_not_retried_after_the_timeout(self):
        calls = []

        async def fetch(url):
            calls.append(url)
            await asyncio.sleep(3600)

        run_bounded(lambda: fetch_all(["slow"], fetch, timeout=0.2, backoff=0))
        self.assertEqual(calls, ["slow"])

    def test_a_caller_can_cancel_the_batch(self):
        async def fetch(url):
            await asyncio.sleep(3600)

        async def scenario():
            task = asyncio.ensure_future(fetch_all(["a", "b"], fetch, timeout=60))
            await asyncio.sleep(0.1)
            task.cancel()
            try:
                await task
            except asyncio.CancelledError:
                return "cancelled"
            return "finished"

        self.assertEqual(run_bounded(scenario), "cancelled")


class EvalHiddenRetries(unittest.TestCase):
    def test_errors_are_still_retried(self):
        calls = []

        async def fetch(url):
            calls.append(url)
            if len(calls) < 3:
                raise ConnectionError("reset")
            return b"ok"

        got = run_bounded(lambda: fetch_all(["a"], fetch, timeout=2, backoff=0))
        self.assertEqual(got, {"a": b"ok"})
        self.assertEqual(len(calls), 3)

    def test_a_host_that_always_fails_gives_none(self):
        async def fetch(url):
            if url == "bad":
                raise OSError("refused")
            return b"ok"

        got = run_bounded(lambda: fetch_all(["good", "bad"], fetch, timeout=2, backoff=0))
        self.assertEqual(got, {"good": b"ok", "bad": None})


if __name__ == "__main__":
    unittest.main()
