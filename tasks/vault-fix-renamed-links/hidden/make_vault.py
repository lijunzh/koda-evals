"""Write this task's fixture/ and solution/ vaults and hidden/expected.json.

Run from the task directory: python3 hidden/make_vault.py

Each note is a template. `{L}` marks a link target that names the renamed
note: the fixture has the old name there, the solution the new one. Text
that isn't a link to it (prose, code, other notes' names) is written out
literally, so it is the same on both sides.
"""

import json
import pathlib
import shutil

OLD, NEW = "Project Atlas", "Atlas Migration"

NOTES = {
    "Home.md": """\
Start here.

## Projects

- [[{L}]]: moving the build farm to the new cluster
- [[Project Atlas Retrospective]]: what we learned from the first attempt
- [[Weekly Notes]]

## Areas

- [[Infra]]
- [[Hiring]]
""",
    "Projects/Atlas Migration.md": """\
Moving the build farm off the old racks and onto the shared cluster.
The [[{L}#Timeline|timeline below]] is the current plan.

## Goals

- One build pool for every team
- Retire the old racks by the end of Q1

## Timeline

| Phase | When | Status |
|---|---|---|
| Pilot with two teams | November | in progress |
| Everyone else | January | planned |
| Racks retired | March | planned |

## Risks

- Cache warm-up after the move doubles build times for a week
- The license server still lives on the old racks

## Related

- [[Project Atlas Retrospective]]
- [[Infra]]
""",
    "Projects/Project Atlas Retrospective.md": """\
What went wrong the first time we tried this, in 2025.

The first Project Atlas attempt stalled when the license server moved. This
note is about that attempt; the current plan is in [[{L}]].

- Project Atlas Retrospective action items are tracked in [[Weekly Notes]].
- See [[{L}#Risks]] for how the license server is handled now.
""",
    "Meetings/2026-10-14 Atlas kickoff.md": """\
Kickoff for [[{L}|Atlas]].

![[{L}#Risks]]

Decisions:
- Pilot teams chosen (see [[{l}]] for the list)
- Weekly sync on Wednesdays
""",
    "Meetings/2026-10-21 Atlas sync.md": """\
Second sync.

- The [plan](../Projects/{U}.md) moved the pilot to November.
- Budget questions for the Project Atlas budget go to finance.
- Open items are in [[Projects/{L}]].
- Not to be confused with [[Project Atlas 2]], which nobody has written yet.
""",
    "Weekly Notes.md": """\
## Week 44

| Project | Note | Status |
|---|---|---|
| Atlas | [[{L}\\|Atlas]] | pilot started |
| Hiring | [[Hiring]] | two offers out |

## Week 43

- Retro follow-ups from [[Project Atlas Retrospective]] are done.
""",
    "Areas/Infra.md": """\
Everything about the build and CI infrastructure.

- Current migration: [[{L}]], due dates in [[{L}#Timeline]].
- Old notes on the racks: [[Rack Inventory]]
""",
    "Areas/Hiring.md": """\
Two open roles on the infra team.

- Build engineer
- Site reliability engineer
""",
    "Areas/Rack Inventory.md": """\
Racks 1 to 6 in the east room. Racks 5 and 6 hold the license server.
""",
    "Guides/Linking Cheatsheet.md": """\
How links work in this vault.

- A link is written `[[Note name]]`, for example `[[Project Atlas|alias]]`.
- A heading link adds `#`: `[[Note#Heading]]`.

```markdown
[[Project Atlas]]
![[Project Atlas#Risks]]
```

Renaming a note in Obsidian updates its links. Renaming it anywhere else does not.
""",
    "Reading/Systems Reading List.md": """\
- A paper on build caching
- A book on running large clusters
- Notes from the scheduling talk
""",
    "Daily/2026-11-02.md": """\
- Reviewed the pilot numbers for [[{L}]].
- Lunch with the hiring panel.
""",
    "Daily/2026-11-03.md": """\
- Nothing on Atlas today.
- Rack 3 lost a power supply; see [[Rack Inventory]].
""",
}

CANVAS = {
    "nodes": [
        {"id": "a1", "type": "file", "file": "Projects/{P}.md", "x": 0, "y": 0, "width": 400, "height": 300},
        {"id": "a2", "type": "file", "file": "Projects/Project Atlas Retrospective.md", "x": 460, "y": 0, "width": 400, "height": 300},
        {"id": "a3", "type": "text", "text": "Project Atlas: pilot first, then everyone.", "x": 0, "y": 360, "width": 400, "height": 120},
    ],
    "edges": [{"id": "e1", "fromNode": "a2", "fromSide": "left", "toNode": "a1", "toSide": "right"}],
}


def render(text: str, name: str) -> str:
    return (
        text.replace("{L}", name)
        .replace("{l}", name.lower())
        .replace("{U}", name.replace(" ", "%20"))
    )


def write(root: pathlib.Path, name: str, only_changed: bool) -> dict:
    out = {}
    for rel, text in NOTES.items():
        body = render(text, name)
        out[rel] = body
        if only_changed and body == render(text, OLD):
            continue
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)
    canvas = json.dumps(CANVAS, indent="\t").replace("{P}", name)
    out["Boards/Atlas.canvas"] = canvas
    path = root / "Boards/Atlas.canvas"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canvas + "\n")
    return out


def main() -> None:
    here = pathlib.Path(".")
    for d in ["fixture", "solution"]:
        shutil.rmtree(here / d, ignore_errors=True)
    write(here / "fixture", OLD, only_changed=False)
    expected = write(here / "solution", NEW, only_changed=True)
    (here / "hidden/expected.json").write_text(json.dumps(expected, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
