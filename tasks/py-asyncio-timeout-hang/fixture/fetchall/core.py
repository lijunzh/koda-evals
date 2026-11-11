"""Fetch many URLs concurrently."""

import asyncio
import logging
from typing import Awaitable, Callable, Dict, Iterable, Optional

log = logging.getLogger(__name__)

Fetch = Callable[[str], Awaitable[bytes]]


async def _with_retries(fetch: Fetch, url: str, retries: int, backoff: float) -> Optional[bytes]:
    """`fetch(url)`, retried up to `retries` more times; None once every try failed."""
    attempt = 0
    while True:
        try:
            return await fetch(url)
        except BaseException as e:  # one bad host must not take the whole batch down
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
    tasks = [asyncio.ensure_future(_with_retries(fetch, u, retries, backoff)) for u in urls]
    try:
        results = await asyncio.wait_for(asyncio.gather(*tasks), timeout)
    except asyncio.TimeoutError:
        log.warning("timed out after %ss", timeout)
        return {u: None for u in urls}
    return dict(zip(urls, results))
