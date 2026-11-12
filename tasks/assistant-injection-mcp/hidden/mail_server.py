#!/usr/bin/env python3
"""A read-only mail server over MCP (stdio), for assistant tasks.

    python3 -I mail_server.py <mailbox dir>

Serves the `.eml` files in the mailbox directory, oldest first by file name,
through two tools:

- `list_messages`: id, sender, subject, date and a 200-character preview
  of the body, as Gmail's API gives a snippet with each message.
- `read_message`: one message's headers and decoded body, by id.

Nothing here writes, sends or reaches the network. Standard library only.
It speaks newline-delimited JSON-RPC 2.0 and answers `initialize` with the
version the client offers when it knows it, else 2025-06-18.
"""

import email
import email.policy
import json
import sys
from pathlib import Path

KNOWN_VERSIONS = {"2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"}
PREVIEW_CHARS = 200

TOOLS = [
    {
        "name": "list_messages",
        "description": "List the messages in the inbox, oldest first: id, from, "
        "subject, date and a short preview of the body.",
        "inputSchema": {"type": "object", "properties": {}},
        "annotations": {"readOnlyHint": True},
    },
    {
        "name": "read_message",
        "description": "Read one message's headers and full body by its id.",
        "inputSchema": {
            "type": "object",
            "properties": {"id": {"type": "string", "description": "The id from list_messages."}},
            "required": ["id"],
        },
        "annotations": {"readOnlyHint": True},
    },
]


def load(mailbox):
    messages = {}
    for path in sorted(Path(mailbox).glob("*.eml")):
        msg = email.message_from_bytes(path.read_bytes(), policy=email.policy.default)
        part = msg.get_body(preferencelist=("plain",))
        body = part.get_content() if part is not None else ""
        messages[path.stem] = {
            "id": path.stem,
            "from": str(msg.get("From", "")),
            "to": str(msg.get("To", "")),
            "subject": str(msg.get("Subject", "")),
            "date": str(msg.get("Date", "")),
            "message_id": str(msg.get("Message-ID", "")),
            "body": body,
        }
    return messages


def preview(body):
    flat = " ".join(body.split())
    return flat if len(flat) <= PREVIEW_CHARS else flat[:PREVIEW_CHARS] + "…"


def call(messages, name, args):
    if name == "list_messages":
        rows = [
            {k: m[k] for k in ("id", "from", "subject", "date")} | {"preview": preview(m["body"])}
            for m in messages.values()
        ]
        return json.dumps(rows, indent=1, ensure_ascii=False), False
    if name == "read_message":
        m = messages.get(str(args.get("id", "")))
        if m is None:
            return f"No message with id {args.get('id')!r}. Ids come from list_messages.", True
        head = "\n".join(
            f"{k.replace('_', '-').title()}: {m[k]}"
            for k in ("from", "to", "subject", "date", "message_id")
        )
        return f"{head}\n\n{m['body']}", False
    return f"Unknown tool {name!r}.", True


def answer(messages, req):
    method = req.get("method")
    params = req.get("params") or {}
    if method == "initialize":
        offered = params.get("protocolVersion")
        return {
            "protocolVersion": offered if offered in KNOWN_VERSIONS else "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "mail", "version": "0.1.0"},
        }
    if method == "ping":
        return {}
    if method == "tools/list":
        return {"tools": TOOLS}
    if method == "tools/call":
        text, is_error = call(messages, params.get("name"), params.get("arguments") or {})
        return {"content": [{"type": "text", "text": text}], "isError": is_error}
    raise LookupError(method)


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: mail_server.py <mailbox dir>")
    messages = load(sys.argv[1])
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "id" not in req:
            continue  # a notification
        try:
            reply = {"jsonrpc": "2.0", "id": req["id"], "result": answer(messages, req)}
        except LookupError as e:
            reply = {
                "jsonrpc": "2.0",
                "id": req["id"],
                "error": {"code": -32601, "message": f"method not found: {e}"},
            }
        sys.stdout.write(json.dumps(reply, ensure_ascii=False) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
