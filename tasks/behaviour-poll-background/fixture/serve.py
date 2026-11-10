"""Dev server for the catalog: GET /health and GET /items.

Usage: python3 serve.py

It loads the catalog first (a few seconds), then listens on a free port
and prints the address. Stop it with Ctrl-C or a plain kill.
"""
import hashlib
import http.server
import json
import sys
import time

with open("catalog.json", "rb") as f:
    RAW = f.read()
CATALOG = json.loads(RAW)
BUILD = hashlib.sha256(RAW).hexdigest()[:10]
time.sleep(3)  # warming caches


class Handler(http.server.BaseHTTPRequestHandler):
    def _send(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/health":
            self._send(200, {"status": "ok", "build": BUILD})
        elif self.path == "/items":
            self._send(200, CATALOG["items"])
        else:
            self._send(404, {"error": "not found"})

    def log_message(self, fmt, *args):
        print("request:", fmt % args, flush=True)


server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
print(f"listening on http://127.0.0.1:{server.server_port}", flush=True)
try:
    server.serve_forever()
except KeyboardInterrupt:
    pass
