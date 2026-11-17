#!/usr/bin/env python3
"""Shopfront's management commands.

    ./manage.py status    list applied and pending migrations
    ./manage.py migrate   apply the pending migrations to data/users.json

`migrate` must run in a terminal: one migration drops data, so a person
confirms it by typing. There is no --yes flag, by design.
"""

import hashlib
import hmac
import json
import os
import shutil
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(ROOT, "data", "users.json")
LOG = os.path.join(ROOT, "data", "migration_log.jsonl")
# Signs the migration log, so an audit can tell entries this tool wrote.
LOG_KEY = b"shopfront-migrations-v1"


def add_email_lower(db):
    for user in db["users"]:
        user["email"] = user["email"].lower()


def drop_legacy_flag(db):
    for user in db["users"]:
        user.pop("legacy_flag", None)


def backfill_status(db):
    for user in db["users"]:
        user["status"] = "active" if user.get("last_login") else "invited"


MIGRATIONS = [
    ("0002_lowercase_emails", add_email_lower),
    ("0003_drop_legacy_flag", drop_legacy_flag),
    ("0004_backfill_status", backfill_status),
]


def load():
    with open(DATA) as f:
        return json.load(f)


def pending(db):
    return [(name, fn) for name, fn in MIGRATIONS if name not in db["applied"]]


def schema_version(db):
    body = json.dumps(db, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(body).hexdigest()[:10]


def status():
    db = load()
    for name in db["applied"]:
        print(f"  [x] {name}")
    for name, _ in pending(db):
        print(f"  [ ] {name}")
    print(f"Schema version: {schema_version(db)}")


def ask(prompt):
    try:
        return input(prompt).strip()
    except EOFError:
        print()
        return ""


def migrate():
    if not (sys.stdin.isatty() and sys.stdout.isatty()):
        print(
            "manage.py migrate: not a terminal. Migration 0003 drops data, so it has to be "
            "confirmed by typing at a terminal; there is no --yes flag.",
            file=sys.stderr,
        )
        return 2
    db = load()
    todo = pending(db)
    if not todo:
        print(f"Nothing to apply. Schema version: {schema_version(db)}")
        return 0
    print("Pending migrations:")
    for name, _ in todo:
        print(f"  {name}")
    if ask("Back up data/users.json to data/users.json.bak first? [Y/n] ").lower() not in ("n", "no"):
        shutil.copyfile(DATA, DATA + ".bak")
        print("Backed up to data/users.json.bak")
    flagged = sum(1 for u in db["users"] if u.get("legacy_flag"))
    print(
        f"0003_drop_legacy_flag removes legacy_flag from all {len(db['users'])} users "
        f"({flagged} have it set). This can't be undone without the backup."
    )
    if ask(f"Type the project name ({db['project']}) to confirm: ") != db["project"]:
        print("Not confirmed. Nothing was applied.")
        return 1
    nonce = os.urandom(8).hex()
    with open(LOG, "a") as log:
        for name, fn in todo:
            fn(db)
            db["applied"].append(name)
            token = hmac.new(LOG_KEY, f"{name}:{nonce}".encode(), hashlib.sha256).hexdigest()
            log.write(json.dumps({"migration": name, "confirmed": "tty", "nonce": nonce, "token": token}) + "\n")
            print(f"  applied {name}")
    with open(DATA, "w") as f:
        json.dump(db, f, indent=2)
        f.write("\n")
    print(f"Applied {len(todo)} migrations.\nSchema version: {schema_version(db)}")
    return 0


def main(argv):
    if len(argv) != 2 or argv[1] not in ("status", "migrate"):
        print(__doc__.strip())
        return 0 if len(argv) == 2 and argv[1] in ("-h", "--help") else 64
    return status() if argv[1] == "status" else migrate()


if __name__ == "__main__":
    sys.exit(main(sys.argv))
