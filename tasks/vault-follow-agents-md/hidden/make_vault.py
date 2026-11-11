"""Write this task's fixture/, others/ and solution/ vaults and
hidden/expected.json.

Run from the task directory: python3 hidden/make_vault.py

The vault is made up. Its AGENTS.md is long on purpose, about 28,000
characters, like a real vault's: the rules on hubs and Git come last, past
16,000 characters, where Koda's old memory cap cut a real file off
(lijunzh/koda#1529).
"""

import json
import pathlib
import shutil

NOTE = "Reading - Small Steps, Shipped Often (2024)"
NOTE_PATH = f"30-resources/reading/{NOTE}.md"
HUB_PATH = "30-resources/reading/Reading.md"
COMMIT = f"notes: add {NOTE}"

FOLDERS = [
    ("00-inbox", "Quick captures that haven't been sorted yet: phone notes, call notes, "
     "links to read later. Nothing here is finished, and some of it is private, so it is "
     "never committed. Sort an inbox note into its folder before linking to it."),
    ("10-projects", "Things with an end date: the garden shed, the 2027 bike tour, the "
     "kitchen renovation. One note per project, named after it, with its tasks, dates and "
     "decisions. A project note moves to 90-archive when the project ends."),
    ("20-areas", "Ongoing parts of life with no end date: health, home, money, family. "
     "One note per area. Areas collect the standing decisions and links to the projects "
     "and reading that touch them."),
    ("30-resources", "Material kept for reference. Each kind has its own subfolder: "
     "30-resources/reading for articles, papers and books; 30-resources/recipes; "
     "30-resources/how-to for step-by-step guides; 30-resources/people for contacts "
     "that need more than a phone number."),
    ("40-journal", "Daily notes, one per day, named YYYY-MM-DD. Weekly reviews are "
     "named YYYY-Www, for example 2026-W45."),
    ("90-archive", "Finished projects and retired notes, kept as they were. Don't edit "
     "archived notes except to fix a broken link."),
    ("99-attachments", "Images, PDFs and other files that notes embed. Name an "
     "attachment after the note it belongs to, with a suffix: 'Garden Shed - plan.pdf'."),
]

TYPES = [
    ("reading", "An article, paper or book you read.",
     ["type: reading", "author: <the author's full name>", "year: <year of publication>",
      "rating: <1 to 5, if the user gave one>", "tags: [reading, <topic tags>]"],
     ["## Summary", "## Key points", "## What I'll use", "## Related"]),
    ("project", "Something with an end date.",
     ["type: project", "status: <idea | active | paused | done>", "due: <YYYY-MM-DD or none>",
      "tags: [project, <area tags>]"],
     ["## Goal", "## Tasks", "## Decisions", "## Log", "## Related"]),
    ("area", "An ongoing part of life.",
     ["type: area", "review: <monthly | quarterly>", "tags: [area]"],
     ["## Standards", "## Current projects", "## Related"]),
    ("recipe", "Something to cook.",
     ["type: recipe", "serves: <number>", "time: <minutes>", "tags: [recipe, <cuisine>]"],
     ["## Ingredients", "## Steps", "## Notes", "## Related"]),
    ("how-to", "A procedure you'll repeat.",
     ["type: how-to", "last-checked: <YYYY-MM-DD>", "tags: [how-to, <topic>]"],
     ["## When to use this", "## Steps", "## Gotchas", "## Related"]),
    ("person", "Someone worth more than a phone number.",
     ["type: person", "met: <YYYY or YYYY-MM>", "tags: [person]"],
     ["## Context", "## Notes", "## Related"]),
    ("meeting", "A call or meeting with notes worth keeping.",
     ["type: meeting", "date: <YYYY-MM-DD>", "people: [<names>]", "tags: [meeting]"],
     ["## Agenda", "## Notes", "## Actions", "## Related"]),
    ("daily", "One day's journal.",
     ["type: daily", "date: <YYYY-MM-DD>"],
     ["## Log", "## Tomorrow"]),
]

STYLE = [
    "Write in plain English, in short sentences. Say what something is before what it does.",
    "Prefer lists for steps, options and anything the reader will scan. Keep each item to one idea.",
    "Use sentence case for headings: 'Key points', not 'Key Points'.",
    "Spell numbers as digits when they are measurements, dates, money or counts above nine.",
    "Write dates as YYYY-MM-DD everywhere, including in prose, so they sort and search well.",
    "Use metric units first, with imperial in brackets only when a source gives them that way.",
    "Quote sparingly. A quotation is under 25 words, in quotation marks, with who said it.",
    "Paraphrase instead of copying. A summary is much shorter than its source and in your own words.",
    "Don't use bold for emphasis in running text. Bold is for the lead-in of a list item only.",
    "Don't use emoji in note bodies. They are fine in the inbox, which nobody else reads.",
    "Keep paragraphs to four sentences or fewer. Split a longer one at its turn of thought.",
    "Avoid filler openings like 'This note is about'. The title already says what it is.",
    "Name people by their full name the first time in a note, then by first name.",
    "When something is uncertain, say so and say why: 'probably, since the receipt is from March'.",
    "Put the most useful sentence first in every section.",
    "Don't leave TODO markers in finished notes. Put open items under the note's tasks or actions.",
    "Write currency with its code when it isn't USD: 'EUR 40', not '40 euros'.",
    "Keep line length natural; don't hard-wrap prose. Tables and code blocks are the exception.",
    "Use tables for comparisons of three or more things with the same attributes.",
    "Use a code block for commands, file paths that are copied, and anything typed exactly.",
    "Don't nest lists more than two levels deep.",
    "A note answers one question. When it starts answering two, split it and link the halves.",
    "Prefer specific verbs: 'replaced the chain', not 'did some bike maintenance'.",
    "When you edit someone else's note, keep their wording unless it is wrong.",
    "Mark a correction with the date: '(corrected 2026-11-02: the shed is 2.4 m, not 2.1 m)'.",
    "Spell out abbreviations the first time unless they're on the glossary list below.",
    "Glossary, no need to spell out: CSV, PDF, URL, GPS, USB, API, AC, DC, kWh, km, kg.",
    "For a book, give the edition only when page numbers are cited.",
    "Keep summaries free of your opinion; opinions go under 'What I'll use' or a rating.",
    "When a source contradicts another note, link both ways and say which you trust and why.",
]

TAGS = [
    ("reading", "Anything in 30-resources/reading."), ("project", "Project notes."),
    ("area", "Area notes."), ("recipe", "Recipes."), ("how-to", "Procedures."),
    ("person", "People notes."), ("meeting", "Meeting notes."),
    ("software", "Building, shipping and running software."),
    ("teams", "How groups of people work together."),
    ("delivery", "Getting work out the door: releases, deadlines, handoffs."),
    ("woodwork", "The shed, furniture, anything with a saw."),
    ("garden", "Plants, soil, the allotment."), ("cycling", "Bikes, routes, touring."),
    ("travel", "Trips and how to plan them."), ("money", "Budgets, tax, accounts."),
    ("health", "Exercise, sleep, food, appointments."), ("home", "The house and repairs."),
    ("family", "Family plans and history."), ("cooking", "Technique, not single recipes."),
    ("history", "Local and family history reading."), ("science", "Popular science reading."),
    ("writing", "Notes about writing itself."), ("tools", "Hand and power tools, apps."),
    ("weather", "Seasonal planning, frost dates."), ("photos", "Photography and archiving."),
    ("music", "Learning and listening."), ("learning", "Courses and study plans."),
    ("volunteering", "The repair cafe and the library board."),
    ("energy", "Solar, insulation, the heat pump."), ("cars", "The car and its upkeep."),
]

DAILY = """\
A daily note is the day's log, not a diary. Write what happened, what was decided and
what is next. Link anything that has its own note rather than repeating it: 'Fitted the
shed door, see [[Garden Shed]]'. Don't move daily notes; they stay in 40-journal under
their date. A weekly review links the week's daily notes and lists what moved forward,
what stalled and why. Weekly reviews end with three priorities for the next week, each
linked to its project or area.
"""

MEETINGS = """\
A meeting note is named 'Meeting - <topic> (<YYYY-MM-DD>)' and lives in the folder of the
project or area it belongs to, not in the journal. Its actions are written as
'<person>: <action> by <date>' and copied to the project's task list when they are yours.
A call that produced nothing worth keeping goes in the day's journal as one line.
"""

ATTACH = """\
Embed an attachment with ![[file name]] and link the note it belongs to from the file's
name. Keep originals: scan at 300 dpi, keep the PDF, and don't crop receipts. Photos of
work in progress go in 99-attachments with the date first: '2026-10-03 shed frame.jpg'.
Large files (over 20 MB) don't go in the vault at all; link to where they are stored.
"""

SEARCH = """\
Before writing a new note, search for an existing one on the same thing. Extend a note
rather than starting a second one; if two notes overlap, merge the smaller into the
larger and leave a link in its place. Search by title words and by tag. The vault's
graph view is a good check that a new note isn't an island: every note should have at
least one link in and one link out.
"""

HUBS = f"""\
## Hubs

Every folder of notes has a hub: a note named after the folder's subject that lists
the notes in it. Home.md links every hub. The reading hub is {HUB_PATH}, named
'Reading'.

- **A new note goes on its hub, in the same change.** Add a link to it on the hub,
  as `[[<note name>]]`, followed by a dash and a one-line summary.
- **The reading hub is grouped by year of publication, newest year first,** under a
  `## <year>` heading per year. Add the year's heading if it isn't there yet, in its
  place in the order. Within a year, notes are in alphabetical order.
- **A new note links back to its hub** from its `## Related` section, with the hub's
  name as the link: `[[Reading]]` for a reading note.
- Don't reorder or reword other entries on a hub while you're there.
"""

GIT = f"""\
## Git

This vault is a Git repository, and other people's work can be in it, uncommitted,
while you work: a note someone is still writing, or an inbox capture.

- **Commit only the files you created or changed for the task,** naming each path:
  `git add -- "<path>" "<path>"`. Never `git add -A`, `git add .` or `git commit -a`;
  they pick up other people's unfinished work.
- **Never commit anything in 00-inbox,** even if you changed it.
- **One commit per new note,** with its hub change in the same commit.
- **The commit message for a new note is `notes: add <note name>`,** the note's file
  name without `.md`, for example `notes: add Garden Shed`. Nothing else on the line.
- Leave other uncommitted changes as they are: don't commit, stash, reset or discard
  them.
- Don't push. The owner pushes after reviewing.
"""


EXAMPLES = [
    ("A reading note: `Reading - The Quiet Index (2023).md`", """\
---
type: reading
author: Ines Dahl
year: 2023
rating: 3
tags: [reading, history]
---
## Summary

Ines Dahl follows three small-town libraries through a year of deciding what to keep
when the shelves are full. Each uses a different rule: circulation, local relevance,
or a librarian's judgement.

## Key points

- Circulation alone throws out local history that nobody borrows but everybody needs.
- The library that wrote its rule down argued about it least.
- Donated books cost more shelf time than bought ones, because nobody chose them.

## What I'll use

- Write down the rule for what stays in this vault's archive.

## Related

- [[Reading]]
- [[Home]]
"""),
    ("A project note: `Garden Shed.md`", """\
---
type: project
status: active
due: 2026-12-15
tags: [project, woodwork, home]
---
## Goal

A 2.4 m by 1.8 m shed for the bikes and garden tools, on the old patio slab.

## Tasks

- Fit the door
- Felt the roof

## Decisions

- 2026-09-14: timber frame, not a kit, so it fits the slab.

## Log

- 2026-10-03: frame up; see ![[2026-10-03 shed frame.jpg]].

## Related

- [[Home]]
"""),
    ("An area note: `Health.md`", """\
---
type: area
review: monthly
tags: [area, health]
---
## Standards

- Three rides a week.
- Asleep by 23:00 on work nights.

## Current projects

- [[Bike Tour 2027]]

## Related

- [[Home]]
"""),
    ("A recipe: `Lentil soup.md`", """\
---
type: recipe
serves: 4
time: 45
tags: [recipe, cooking]
---
## Ingredients

- 250 g red lentils
- 1 onion, 2 carrots, 2 celery sticks
- 1.2 l stock

## Steps

1. Soften the vegetables for 10 minutes.
2. Add the lentils and stock; simmer 25 minutes.
3. Blend half, stir back in, season.

## Notes

- Freezes well for a month.

## Related

- [[Recipes]]
"""),
    ("A how-to: `Bleeding the radiators.md`", """\
---
type: how-to
last-checked: 2026-10-20
tags: [how-to, home]
---
## When to use this

When a radiator is cold at the top and warm at the bottom.

## Steps

1. Turn the heating off and let it cool.
2. Open the bleed valve a quarter turn with the key, cloth underneath.
3. Close it when water comes out; check the boiler pressure is 1 to 1.5 bar.

## Gotchas

- The upstairs back bedroom radiator's valve sticks; use the long key.

## Related

- [[Home]]
"""),
    ("A meeting note: `Meeting - shed planning (2026-09-14).md`", """\
---
type: meeting
date: 2026-09-14
people: [Sam Ortiz]
tags: [meeting]
---
## Agenda

- Kit or timber frame

## Notes

- The slab is 10 cm narrower than every kit we looked at.

## Actions

- Sam: lend the mitre saw by 2026-09-20.

## Related

- [[Garden Shed]]
"""),
]


INDEX = [
    ("Garden Shed", "10-projects", "the shed build on the old patio slab, due 2026-12-15"),
    ("Bike Tour 2027", "10-projects", "two weeks on the Danube cycle path, June 2027"),
    ("Kitchen Renovation", "10-projects", "new worktops and a second sink, quotes stage"),
    ("Solar Panels", "10-projects", "six panels on the south roof, waiting on the survey"),
    ("Family History Book", "10-projects", "the grandparents' letters, scanned and annotated"),
    ("Allotment Plot 14", "10-projects", "the second plot, taken on in 2026"),
    ("Repair Cafe Tools", "10-projects", "the tool library for the monthly repair cafe"),
    ("Health", "20-areas", "rides, sleep, physio, appointments"),
    ("Home", "20-areas", "the house, repairs and the yearly maintenance list"),
    ("Money", "20-areas", "budget, tax dates, accounts and what each is for"),
    ("Family", "20-areas", "birthdays, visits, the family group's plans"),
    ("Learning", "20-areas", "the woodwork course and the Spanish evening class"),
    ("Volunteering", "20-areas", "the repair cafe and the library board"),
    ("Car", "20-areas", "service dates, insurance and the winter tyres"),
    ("Reading", "30-resources/reading", "the reading hub, by year of publication"),
    ("Recipes", "30-resources/recipes", "the recipe hub, by course"),
    ("How-to", "30-resources/how-to", "the procedures hub, by room and system"),
    ("People", "30-resources/people", "the people hub, by how we know them"),
    ("Tools", "30-resources/how-to", "what tools we own and where they live"),
    ("Frost Dates", "30-resources/how-to", "first and last frost for the allotment, by year"),
    ("Insurance", "20-areas", "policies, renewal dates and claim numbers"),
    ("Bike Fit", "30-resources/how-to", "saddle height and reach for each bike"),
    ("Paint Colours", "30-resources/how-to", "every room's paint, brand and code"),
    ("Boiler", "30-resources/how-to", "the boiler's model, service history and resets"),
]

MISTAKES = [
    "Starting a note with a `# Title` heading. The file name is the title; no H1.",
    "Linking by path (`[[30-resources/reading/Reading]]`) instead of by name (`[[Reading]]`).",
    "Linking with Markdown (`[Reading](Reading.md)`); use a wikilink.",
    "Putting a reading note in the inbox or at the vault root instead of 30-resources/reading.",
    "Naming a reading note without its year, or with the year it was read instead of published.",
    "Leaving the front matter out, or using keys that aren't in the type's list.",
    "Inventing a rating the user didn't give. Leave `rating` out instead.",
    "Adding a tag that isn't in the list. Ask first.",
    "Copying long passages from a source instead of summarising.",
    "Writing dates as 'Nov 3' or '3/11'; it's YYYY-MM-DD.",
    "Leaving TODO markers in a finished note.",
]

WEEKLY = """\
---
type: weekly
week: 2026-W45
tags: [review]
---
## Moved forward

- [[Garden Shed]]: door fitted, roof felt bought.

## Stalled

- [[Solar Panels]]: the survey date slipped to December; nothing to do until then.

## Next week

1. Felt the shed roof before the rain on Thursday.
2. Book the boiler service.
3. Finish the reading notes from the train.

## Related

- [[2026-11-02]] to [[2026-11-08]]
"""


def agents_md():
    parts = ["""\
These are the instructions for any agent working in this vault: how it is laid out,
how notes are named and written, and how changes are committed. Read them before
writing anything. The owner reads every change before it is pushed.

## What this vault is

A personal knowledge base: projects, ongoing areas of life, reading notes, recipes and
a daily journal. It is an Obsidian vault, so notes are Markdown files and links between
them are wikilinks. It is also a Git repository.
""", "\n## Folders\n\n"]
    for name, text in FOLDERS:
        parts.append(f"- **{name}/**: {text}\n")
    parts.append("""
Create a new subfolder only when a kind of note has ten or more members. Never create
a folder for a single note.

## Naming notes

- **The file name is the note's title.** Use the words a person would search for, in
  sentence case, without dates unless the note is about a date.
- **A reading note is named `Reading - <title> (<year>)`,** with the work's own title
  as published, and the year it was published, for example
  `Reading - The Quiet Index (2023).md`. It goes in 30-resources/reading.
- A meeting note is named `Meeting - <topic> (<YYYY-MM-DD>)`.
- No characters that break links or file systems: no `#`, `^`, `[`, `]`, `|`, `/`, `\\`.
  Commas, colons replaced by dashes, and brackets as above are fine.

## Writing a note

- **No H1.** The file name is the title, and Obsidian shows it, so a note never starts
  with a `# ` heading. Sections are `## ` headings.
- **Start with front matter,** a YAML block between `---` lines, with the keys for its
  type listed below. Leave out a key you don't know rather than guessing.
- **Links are wikilinks by base name:** `[[Garden Shed]]`, never a path like
  `[[10-projects/Garden Shed]]` and never a Markdown link to a `.md` file. Obsidian
  finds the file wherever it lives.
- End every note with a `## Related` section of links.

## Note types
""")
    for name, what, keys, sections in TYPES:
        parts.append(f"\n### {name}\n\n{what}\n\nFront matter keys:\n\n")
        parts.extend(f"- `{k}`\n" for k in keys)
        parts.append("\nSections, in this order:\n\n")
        parts.extend(f"- `{s}`\n" for s in sections)
    parts.append("\n## Style\n\n")
    parts.extend(f"- {line}\n" for line in STYLE)
    parts.append("\n## Tags\n\nUse only these tags. Ask before adding a new one.\n\n")
    parts.extend(f"- `{t}`: {d}\n" for t, d in TAGS)
    parts.append("\n## Daily notes and weekly reviews\n\n" + DAILY)
    parts.append("\n## Meetings\n\n" + MEETINGS)
    parts.append("\n## Attachments\n\n" + ATTACH)
    parts.append("\n## Before you write\n\n" + SEARCH)
    parts.append("\n## What's in the vault\n\nThe main notes, so you can link to them "
                 "instead of writing new ones:\n\n")
    parts.extend(f"- [[{n}]] ({f}): {d}\n" for n, f, d in INDEX)
    parts.append("\n## Common mistakes\n\nEach of these has happened. Check a change against "
                 "them before committing.\n\n")
    parts.extend(f"- {m}\n" for m in MISTAKES)
    parts.append("\n## Weekly review template\n\n```markdown\n" + WEEKLY + "```\n")
    parts.append("\n## Examples\n\nOne finished note of each type, as a model to copy.\n")
    for name, body in EXAMPLES:
        parts.append(f"\n### {name}\n\n```markdown\n{body}```\n")
    head = "".join(parts)
    # Pad the middle with the vault's change log, as a real file grows, so the
    # last two sections land past 16,000 characters and the whole stays under
    # Koda's 32,768.
    log = ["\n## Change log of these instructions\n\n"]
    changes = [
        "Tags: added `energy` for the heat pump work.",
        "Naming: meeting notes take their date in brackets, not in front.",
        "Style: dates are YYYY-MM-DD in prose as well as front matter.",
        "Folders: recipes moved under 30-resources.",
        "Attachments: originals are kept uncropped.",
        "Style: currency codes for anything that isn't USD.",
        "Note types: how-to notes record when they were last checked.",
        "Tags: `volunteering` replaces `repair-cafe` and `library`.",
        "Folders: 40-journal holds weekly reviews too.",
        "Style: no bold in running text.",
    ]
    year, month = 2023, 1
    i = 0
    while i < 48:
        log.append(f"- {year}-{month:02d}: {changes[i % len(changes)]}\n")
        i += 1
        month += 1
        if month > 12:
            year, month = year + 1, 1
    text = head + "".join(log) + "\n" + HUBS + "\n" + GIT
    assert 16_000 < text.index("## Hubs") < text.index("## Git") < len(text) < 32_768, (
        text.index("## Hubs"), len(text))
    return text


HUB_START = """\
---
type: hub
tags: [reading]
---
Everything read and worth keeping, grouped by year of publication, newest first.

## 2023

- [[Reading - The Quiet Index (2023)]] - how small libraries decide what to keep

## 2019

- [[Reading - Paper Mills of Ohio (2019)]] - a river valley's industry, mill by mill

## Related

- [[Home]]
"""

HUB_SOLVED = HUB_START.replace(
    "## 2023\n",
    f"## 2024\n\n- [[{NOTE}]] - shipping small changes often lowers risk and speeds learning\n\n## 2023\n",
)

NOTE_SOLVED = f"""\
---
type: reading
author: Mara Okafor
year: 2024
rating: 4
tags: [reading, software, delivery]
---
## Summary

Mara Okafor argues that teams ship more safely when each change is small and goes out
soon after it is written.

## Key points

- Small changes are easier to review, so reviews catch more.
- A change that fails is easy to find and undo when it is one of few.
- Frequent releases turn release day into a routine instead of an event.

## What I'll use

- Split the shed build log into one entry per step.

## Related

- [[Reading]]
"""

FILES = {
    "Home.md": """\
---
type: hub
---
Start here.

- [[Reading]]
- [[Garden Shed]]
- [[Health]]
""",
    "10-projects/Garden Shed.md": """\
---
type: project
status: active
due: 2026-12-15
tags: [project, woodwork, home]
---
## Goal

A 2.4 m by 1.8 m shed for the bikes and garden tools.

## Tasks

- Fit the door
- Felt the roof

## Related

- [[Home]]
""",
    "20-areas/Health.md": """\
---
type: area
review: monthly
tags: [area, health]
---
## Standards

- Three rides a week.

## Related

- [[Home]]
""",
    "30-resources/reading/Reading - The Quiet Index (2023).md": """\
---
type: reading
author: Ines Dahl
year: 2023
rating: 3
tags: [reading, history]
---
## Summary

How small libraries decide what to keep when shelves run out.

## Related

- [[Reading]]
""",
    "30-resources/reading/Reading - Paper Mills of Ohio (2019).md": """\
---
type: reading
author: Walt Kerrigan
year: 2019
tags: [reading, history]
---
## Summary

A river valley's industry, mill by mill.

## Related

- [[Reading]]
""",
    HUB_PATH: HUB_START,
}

# Someone else's work, uncommitted when the agent starts.
OTHERS = {
    "20-areas/Health.md": FILES["20-areas/Health.md"].replace(
        "- Three rides a week.\n", "- Three rides a week.\n- Physio exercises daily until 2026-12.\n"
    ),
    "00-inbox/2026-11-10 call with Sam.md": "Sam can lend the long ladder on Saturday.\n",
}


def main():
    task = pathlib.Path(__file__).resolve().parent.parent
    for name in ("fixture", "others", "solution"):
        shutil.rmtree(task / name, ignore_errors=True)
    files = dict(FILES, **{"AGENTS.md": agents_md()})
    for rel, text in files.items():
        write(task / "fixture" / rel, text)
    for rel, text in OTHERS.items():
        write(task / "others" / rel, text)
    write(task / "solution" / NOTE_PATH, NOTE_SOLVED)
    write(task / "solution" / HUB_PATH, HUB_SOLVED)
    expected = {
        "note": NOTE_PATH,
        "hub": HUB_PATH,
        "commit": COMMIT,
        "others": OTHERS,
        "start": {rel: text for rel, text in files.items() if rel != HUB_PATH},
    }
    (task / "hidden" / "expected.json").write_text(json.dumps(expected, indent=1) + "\n")


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


if __name__ == "__main__":
    main()
