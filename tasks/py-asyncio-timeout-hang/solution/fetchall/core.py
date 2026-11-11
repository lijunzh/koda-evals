"""Fetch many URLs concurrently."""

import asyncio
import logging
from typing import Awaitable, Callable, Dict, Iterable, Optional

log = logging.getLogger(__name__)

Fetch = Callable[[str], Awaitable[bytes]]


async def _with_retries(fetch: Fetch, url: str, retries: int, backoff: float) -> Optional[bytes]:
    """`fetch(url)`, retried up to `retries` more times; None once every try failed.

    Only errors are retried. Cancellation (the timeout, or the caller giving
    up) is a `BaseException`, not an `Exception`, and must get through.
    """
    attempt = 0
    while True:
        try:
            return await fetch(url)
        except Exception as e:  # one bad host must not take the whole batch down
            attempt += 1
            if attempt > retries:
                log.warning("giving up on %s: %r", url, e)
                return None
            log.info("retrying %s after %r", url, e)
            await asyncio.sleep(backoff * attempt)


async def fetch_all(
    urls: Iterable[str],
    fetch: Fetch,
    *,
    timeout: float,
    retries: int = 2,
    backoff: float = 0.05,
) -> Dict[str, Optional[bytes]]:
    """Fetch every URL at once.

    After `timeout` seconds, stop waiting: cancel the fetches still running
    and return what finished, with None for the rest and for URLs that failed
    every retry.
    """
    urls = list(dict.fromkeys(urls))
    tasks = {u: asyncio.ensure_future(_with_retries(fetch, u, retries, backoff)) for u in urls}
    try:
        _, pending = await asyncio.wait(tasks.values(), timeout=timeout)
    finally:
        # On the timeout, and when the caller cancels us, nothing is left running.
        for t in tasks.values():
            t.cancel()
        await asyncio.gather(*tasks.values(), return_exceptions=True)
    if pending:
        log.warning("timed out after %ss", timeout)
    return {
        u: None if t.cancelled() or t.exception() is not None else t.result()
        for u, t in tasks.items()
    }
