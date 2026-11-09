"""Fetch `/` from every host:port at once: python3 collect.py --timeout SECS HOST:PORT..."""

import argparse
import asyncio
import logging
import sys

from fetchall import fetch_all


async def http_get(target: str) -> bytes:
    host, _, port = target.rpartition(":")
    reader, writer = await asyncio.open_connection(host, int(port))
    try:
        writer.write(f"GET / HTTP/1.0\r\nHost: {host}\r\n\r\n".encode())
        await writer.drain()
        return await reader.read(65536)
    finally:
        writer.close()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--timeout", type=float, default=5.0)
    ap.add_argument("targets", nargs="+")
    args = ap.parse_args(argv)
    logging.basicConfig(level=logging.WARNING, format="%(levelname)s %(message)s")
    results = asyncio.run(fetch_all(args.targets, http_get, timeout=args.timeout))
    for target, body in results.items():
        status = body.split(b"\r\n", 1)[0].decode(errors="replace") if body else "no answer"
        print(f"{target:<30} {status}")
    return 0 if all(results.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
