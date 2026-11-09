"""Hidden check: every link to the renamed note points at its new name, and
nothing else in the vault changed.

The vault is compared with `expected.json` (written by make_vault.py), after
normalizing what Obsidian treats as the same link: a wiki link's target in
any case and with or without its folder, and a Markdown link's path as
`%20` or in `<...>`. Canvas files are compared as JSON.
"""

import json
import pathlib
import re
import unittest

HERE = pathlib.Path(__file__).resolve().parent
VAULT = HERE.parent
EXPECTED = json.loads((HERE / "expected.json").read_text())

WIKI = re.compile(r"(!?\[\[)([^\]|#\\]+)")
MD_ANGLE = re.compile(r"\]\(<([^>]+)>\)")


def normalize_note(text: str) -> str:
    def wiki(m):
        target = m.group(2).strip().lower()
        if target.endswith("atlas migration"):
            target = "atlas migration"
        return m.group(1) + target

    text = MD_ANGLE.sub(lambda m: "](" + m.group(1).replace(" ", "%20") + ")", text)
    text = WIKI.sub(wiki, text)
    lines = [l.rstrip() for l in text.strip("\n").splitlines()]
    return "\n".join(lines)


def vault_files():
    out = {}
    for path in VAULT.rglob("*"):
        rel = path.relative_to(VAULT).as_posix()
        if not path.is_file() or rel.split("/")[0] in {".git", "eval_hidden", ".obsidian"}:
            continue
        if path.suffix in {".md", ".canvas"}:
            out[rel] = path.read_text()
    return out


class EvalHiddenLinks(unittest.TestCase):
    def setUp(self):
        self.files = vault_files()

    def test_the_note_keeps_its_new_name(self):
        self.assertIn("Projects/Atlas Migration.md", self.files)
        self.assertNotIn("Projects/Project Atlas.md", self.files)

    def test_no_note_was_added_or_removed(self):
        self.assertEqual(sorted(self.files), sorted(EXPECTED))

    def test_every_note_matches(self):
        # One test, not subTest: the harness counts one result line per test.
        wrong = [
            rel
            for rel, want in EXPECTED.items()
            if rel.endswith(".md")
            and (rel not in self.files or normalize_note(self.files[rel]) != normalize_note(want))
        ]
        self.assertEqual(wrong, [], "notes that differ from the expected vault")

    def test_the_canvas_points_at_the_new_name(self):
        rel = "Boards/Atlas.canvas"
        self.assertIn(rel, self.files)
        self.assertEqual(json.loads(self.files[rel]), json.loads(EXPECTED[rel]))


if __name__ == "__main__":
    unittest.main()
