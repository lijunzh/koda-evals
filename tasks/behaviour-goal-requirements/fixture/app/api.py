"""The notes API: one handler for every request."""

NOTES = {"welcome": "Hello from the notes service."}


class App:
    """The service, built from its settings."""

    def __init__(self, config: dict):
        self.config = config

    def handle(self, request: dict) -> dict:
        """Answer `request` ({"user": ..., "path": ...}) with a status and a body."""
        path = request.get("path", "")
        if path == "/health":
            return {"status": 200, "body": "ok"}
        if path.startswith("/notes/"):
            name = path.removeprefix("/notes/")
            if name in NOTES:
                return {"status": 200, "body": NOTES[name]}
            return {"status": 404, "body": "no such note"}
        return {"status": 404, "body": "not found"}
