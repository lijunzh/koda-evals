"""The notes API: one handler for every request."""

from collections import defaultdict, deque

from app import clock

NOTES = {"welcome": "Hello from the notes service."}


class App:
    """The service, built from its settings."""

    def __init__(self, config: dict):
        self.config = config
        self.limit = config.get("rate_limit_per_minute", 60)
        self.recent: dict[str, deque] = defaultdict(deque)

    def handle(self, request: dict) -> dict:
        """Answer `request` ({"user": ..., "path": ...}) with a status and a body."""
        if self.limited(request.get("user", "")):
            return {"status": 429, "body": "too many requests"}
        path = request.get("path", "")
        if path == "/health":
            return {"status": 200, "body": "ok"}
        if path.startswith("/notes/"):
            name = path.removeprefix("/notes/")
            if name in NOTES:
                return {"status": 200, "body": NOTES[name]}
            return {"status": 404, "body": "no such note"}
        return {"status": 404, "body": "not found"}

    def limited(self, user: str) -> bool:
        """Whether `user` has used up the requests allowed in the last minute."""
        now = clock.now()
        recent = self.recent[user]
        while recent and now - recent[0] >= 60:
            recent.popleft()
        if len(recent) >= self.limit:
            return True
        recent.append(now)
        return False
