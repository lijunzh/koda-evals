"""Hidden check: the reading note follows the vault's AGENTS.md, and the
commit holds only it and the hub, leaving someone else's work uncommitted.

Every assertion is a rule in the fixture's AGENTS.md or a fact in the prompt.
`expected.json` is written by make_vault.py.
"""

import json
import pathlib
import re
import subprocess
import unittest

HERE = pathlib.Path(__file__).resolve().parent
VAULT = HERE.parent
EXPECTED = json.loads((HERE / "expected.json").read_text())
NOTE, HUB = EXPECTED["note"], EXPECTED["hub"]
NOTE_NAME = pathlib.PurePosixPath(NOTE).stem


def git(*args):
    return subprocess.run(
        ["git", "-C", str(VAULT), *args], capture_output=True, text=True, check=True
    ).stdout


def new_commits():
    """The commits after the start commit, oldest first."""
    root = git("rev-list", "--max-parents=0", "HEAD").split()[-1]
    return git("rev-list", "--reverse", f"{root}..HEAD").split()


def read(rel):
    return (VAULT / rel).read_text()


def front_matter(text):
    m = re.match(r"---\n(.*?)\n---\n", text, re.S)
    return m.group(1) if m else ""


def section(text, heading):
    """The lines under `## heading`, up to the next `## `."""
    m = re.search(rf"^## {re.escape(heading)}\s*\n(.*?)(?=^## |\Z)", text, re.M | re.S)
    return m.group(1) if m else None


class TheNote(unittest.TestCase):
    def test_is_named_and_placed_as_agents_md_says(self):
        self.assertTrue((VAULT / NOTE).is_file(), f"no {NOTE}")

    def test_has_no_h1(self):
        self.assertIsNone(re.search(r"^# ", read(NOTE), re.M), "the note has an H1")

    def test_front_matter_has_the_reading_keys(self):
        fm = front_matter(read(NOTE))
        self.assertRegex(fm, r"(?m)^type:\s*reading\s*$")
        self.assertRegex(fm, r"(?m)^author:\s*['\"]?Mara Okafor['\"]?\s*$")
        self.assertRegex(fm, r"(?m)^year:\s*['\"]?2024['\"]?\s*$")
        self.assertRegex(fm, r"(?m)^rating:\s*['\"]?4(/5)?['\"]?\s*$")

    def test_links_back_to_its_hub_by_name(self):
        related = section(read(NOTE), "Related")
        self.assertIsNotNone(related, "no ## Related section")
        self.assertIn("[[Reading]]", related)

    def test_links_are_wikilinks_by_name(self):
        text = read(NOTE)
        self.assertNotRegex(text, r"\[\[[^\]]*/", "a link by path")
        self.assertNotRegex(text, r"\]\([^)]*\.md\)", "a Markdown link to a note")


class TheHub(unittest.TestCase):
    def test_lists_the_note_under_its_year_newest_first(self):
        hub = read(HUB)
        year = section(hub, "2024")
        self.assertIsNotNone(year, "no ## 2024 heading on the hub")
        self.assertIn(f"[[{NOTE_NAME}]]", year)
        self.assertLess(hub.index("## 2024"), hub.index("## 2023"), "2024 isn't first")

    def test_keeps_the_other_entries(self):
        hub = read(HUB)
        for name in ("Reading - The Quiet Index (2023)", "Reading - Paper Mills of Ohio (2019)"):
            self.assertIn(f"[[{name}]]", hub)
        self.assertIn("[[Home]]", hub)


class TheCommit(unittest.TestCase):
    def test_one_commit_with_the_message_agents_md_gives(self):
        commits = new_commits()
        self.assertEqual(len(commits), 1, f"{len(commits)} commits, not one")
        message = git("log", "-1", "--format=%s", commits[0]).strip()
        self.assertEqual(message, EXPECTED["commit"])

    def test_holds_only_the_note_and_the_hub(self):
        files = set()
        for commit in new_commits():
            files |= set(git("show", "--name-only", "--format=", commit).split("\n")) - {""}
        self.assertEqual(files, {NOTE, HUB})


class OthersWork(unittest.TestCase):
    def test_is_still_there_and_uncommitted(self):
        for rel, text in EXPECTED["others"].items():
            self.assertEqual(read(rel), text, f"{rel} changed")
            committed = subprocess.run(
                ["git", "-C", str(VAULT), "cat-file", "-e", f"HEAD:{rel}"],
                capture_output=True,
            ).returncode == 0
            if committed:
                self.assertNotEqual(git("show", f"HEAD:{rel}"), text, f"{rel} was committed")
            else:
                self.assertNotIn(rel, git("ls-tree", "-r", "--name-only", "HEAD").split("\n"))

    def test_nothing_else_changed(self):
        for rel, text in EXPECTED["start"].items():
            want = EXPECTED["others"].get(rel, text)
            self.assertEqual(read(rel), want, f"{rel} changed")


if __name__ == "__main__":
    unittest.main()
