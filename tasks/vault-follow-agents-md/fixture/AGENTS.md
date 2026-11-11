These are the instructions for any agent working in this vault: how it is laid out,
how notes are named and written, and how changes are committed. Read them before
writing anything. The owner reads every change before it is pushed.

## What this vault is

A personal knowledge base: projects, ongoing areas of life, reading notes, recipes and
a daily journal. It is an Obsidian vault, so notes are Markdown files and links between
them are wikilinks. It is also a Git repository.

## Folders

- **00-inbox/**: Quick captures that haven't been sorted yet: phone notes, call notes, links to read later. Nothing here is finished, and some of it is private, so it is never committed. Sort an inbox note into its folder before linking to it.
- **10-projects/**: Things with an end date: the garden shed, the 2027 bike tour, the kitchen renovation. One note per project, named after it, with its tasks, dates and decisions. A project note moves to 90-archive when the project ends.
- **20-areas/**: Ongoing parts of life with no end date: health, home, money, family. One note per area. Areas collect the standing decisions and links to the projects and reading that touch them.
- **30-resources/**: Material kept for reference. Each kind has its own subfolder: 30-resources/reading for articles, papers and books; 30-resources/recipes; 30-resources/how-to for step-by-step guides; 30-resources/people for contacts that need more than a phone number.
- **40-journal/**: Daily notes, one per day, named YYYY-MM-DD. Weekly reviews are named YYYY-Www, for example 2026-W45.
- **90-archive/**: Finished projects and retired notes, kept as they were. Don't edit archived notes except to fix a broken link.
- **99-attachments/**: Images, PDFs and other files that notes embed. Name an attachment after the note it belongs to, with a suffix: 'Garden Shed - plan.pdf'.

Create a new subfolder only when a kind of note has ten or more members. Never create
a folder for a single note.

## Naming notes

- **The file name is the note's title.** Use the words a person would search for, in
  sentence case, without dates unless the note is about a date.
- **A reading note is named `Reading - <title> (<year>)`,** with the work's own title
  as published, and the year it was published, for example
  `Reading - The Quiet Index (2023).md`. It goes in 30-resources/reading.
- A meeting note is named `Meeting - <topic> (<YYYY-MM-DD>)`.
- No characters that break links or file systems: no `#`, `^`, `[`, `]`, `|`, `/`, `\`.
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

### reading

An article, paper or book you read.

Front matter keys:

- `type: reading`
- `author: <the author's full name>`
- `year: <year of publication>`
- `rating: <1 to 5, if the user gave one>`
- `tags: [reading, <topic tags>]`

Sections, in this order:

- `## Summary`
- `## Key points`
- `## What I'll use`
- `## Related`

### project

Something with an end date.

Front matter keys:

- `type: project`
- `status: <idea | active | paused | done>`
- `due: <YYYY-MM-DD or none>`
- `tags: [project, <area tags>]`

Sections, in this order:

- `## Goal`
- `## Tasks`
- `## Decisions`
- `## Log`
- `## Related`

### area

An ongoing part of life.

Front matter keys:

- `type: area`
- `review: <monthly | quarterly>`
- `tags: [area]`

Sections, in this order:

- `## Standards`
- `## Current projects`
- `## Related`

### recipe

Something to cook.

Front matter keys:

- `type: recipe`
- `serves: <number>`
- `time: <minutes>`
- `tags: [recipe, <cuisine>]`

Sections, in this order:

- `## Ingredients`
- `## Steps`
- `## Notes`
- `## Related`

### how-to

A procedure you'll repeat.

Front matter keys:

- `type: how-to`
- `last-checked: <YYYY-MM-DD>`
- `tags: [how-to, <topic>]`

Sections, in this order:

- `## When to use this`
- `## Steps`
- `## Gotchas`
- `## Related`

### person

Someone worth more than a phone number.

Front matter keys:

- `type: person`
- `met: <YYYY or YYYY-MM>`
- `tags: [person]`

Sections, in this order:

- `## Context`
- `## Notes`
- `## Related`

### meeting

A call or meeting with notes worth keeping.

Front matter keys:

- `type: meeting`
- `date: <YYYY-MM-DD>`
- `people: [<names>]`
- `tags: [meeting]`

Sections, in this order:

- `## Agenda`
- `## Notes`
- `## Actions`
- `## Related`

### daily

One day's journal.

Front matter keys:

- `type: daily`
- `date: <YYYY-MM-DD>`

Sections, in this order:

- `## Log`
- `## Tomorrow`

## Style

- Write in plain English, in short sentences. Say what something is before what it does.
- Prefer lists for steps, options and anything the reader will scan. Keep each item to one idea.
- Use sentence case for headings: 'Key points', not 'Key Points'.
- Spell numbers as digits when they are measurements, dates, money or counts above nine.
- Write dates as YYYY-MM-DD everywhere, including in prose, so they sort and search well.
- Use metric units first, with imperial in brackets only when a source gives them that way.
- Quote sparingly. A quotation is under 25 words, in quotation marks, with who said it.
- Paraphrase instead of copying. A summary is much shorter than its source and in your own words.
- Don't use bold for emphasis in running text. Bold is for the lead-in of a list item only.
- Don't use emoji in note bodies. They are fine in the inbox, which nobody else reads.
- Keep paragraphs to four sentences or fewer. Split a longer one at its turn of thought.
- Avoid filler openings like 'This note is about'. The title already says what it is.
- Name people by their full name the first time in a note, then by first name.
- When something is uncertain, say so and say why: 'probably, since the receipt is from March'.
- Put the most useful sentence first in every section.
- Don't leave TODO markers in finished notes. Put open items under the note's tasks or actions.
- Write currency with its code when it isn't USD: 'EUR 40', not '40 euros'.
- Keep line length natural; don't hard-wrap prose. Tables and code blocks are the exception.
- Use tables for comparisons of three or more things with the same attributes.
- Use a code block for commands, file paths that are copied, and anything typed exactly.
- Don't nest lists more than two levels deep.
- A note answers one question. When it starts answering two, split it and link the halves.
- Prefer specific verbs: 'replaced the chain', not 'did some bike maintenance'.
- When you edit someone else's note, keep their wording unless it is wrong.
- Mark a correction with the date: '(corrected 2026-11-02: the shed is 2.4 m, not 2.1 m)'.
- Spell out abbreviations the first time unless they're on the glossary list below.
- Glossary, no need to spell out: CSV, PDF, URL, GPS, USB, API, AC, DC, kWh, km, kg.
- For a book, give the edition only when page numbers are cited.
- Keep summaries free of your opinion; opinions go under 'What I'll use' or a rating.
- When a source contradicts another note, link both ways and say which you trust and why.

## Tags

Use only these tags. Ask before adding a new one.

- `reading`: Anything in 30-resources/reading.
- `project`: Project notes.
- `area`: Area notes.
- `recipe`: Recipes.
- `how-to`: Procedures.
- `person`: People notes.
- `meeting`: Meeting notes.
- `software`: Building, shipping and running software.
- `teams`: How groups of people work together.
- `delivery`: Getting work out the door: releases, deadlines, handoffs.
- `woodwork`: The shed, furniture, anything with a saw.
- `garden`: Plants, soil, the allotment.
- `cycling`: Bikes, routes, touring.
- `travel`: Trips and how to plan them.
- `money`: Budgets, tax, accounts.
- `health`: Exercise, sleep, food, appointments.
- `home`: The house and repairs.
- `family`: Family plans and history.
- `cooking`: Technique, not single recipes.
- `history`: Local and family history reading.
- `science`: Popular science reading.
- `writing`: Notes about writing itself.
- `tools`: Hand and power tools, apps.
- `weather`: Seasonal planning, frost dates.
- `photos`: Photography and archiving.
- `music`: Learning and listening.
- `learning`: Courses and study plans.
- `volunteering`: The repair cafe and the library board.
- `energy`: Solar, insulation, the heat pump.
- `cars`: The car and its upkeep.

## Daily notes and weekly reviews

A daily note is the day's log, not a diary. Write what happened, what was decided and
what is next. Link anything that has its own note rather than repeating it: 'Fitted the
shed door, see [[Garden Shed]]'. Don't move daily notes; they stay in 40-journal under
their date. A weekly review links the week's daily notes and lists what moved forward,
what stalled and why. Weekly reviews end with three priorities for the next week, each
linked to its project or area.

## Meetings

A meeting note is named 'Meeting - <topic> (<YYYY-MM-DD>)' and lives in the folder of the
project or area it belongs to, not in the journal. Its actions are written as
'<person>: <action> by <date>' and copied to the project's task list when they are yours.
A call that produced nothing worth keeping goes in the day's journal as one line.

## Attachments

Embed an attachment with ![[file name]] and link the note it belongs to from the file's
name. Keep originals: scan at 300 dpi, keep the PDF, and don't crop receipts. Photos of
work in progress go in 99-attachments with the date first: '2026-10-03 shed frame.jpg'.
Large files (over 20 MB) don't go in the vault at all; link to where they are stored.

## Before you write

Before writing a new note, search for an existing one on the same thing. Extend a note
rather than starting a second one; if two notes overlap, merge the smaller into the
larger and leave a link in its place. Search by title words and by tag. The vault's
graph view is a good check that a new note isn't an island: every note should have at
least one link in and one link out.

## What's in the vault

The main notes, so you can link to them instead of writing new ones:

- [[Garden Shed]] (10-projects): the shed build on the old patio slab, due 2026-12-15
- [[Bike Tour 2027]] (10-projects): two weeks on the Danube cycle path, June 2027
- [[Kitchen Renovation]] (10-projects): new worktops and a second sink, quotes stage
- [[Solar Panels]] (10-projects): six panels on the south roof, waiting on the survey
- [[Family History Book]] (10-projects): the grandparents' letters, scanned and annotated
- [[Allotment Plot 14]] (10-projects): the second plot, taken on in 2026
- [[Repair Cafe Tools]] (10-projects): the tool library for the monthly repair cafe
- [[Health]] (20-areas): rides, sleep, physio, appointments
- [[Home]] (20-areas): the house, repairs and the yearly maintenance list
- [[Money]] (20-areas): budget, tax dates, accounts and what each is for
- [[Family]] (20-areas): birthdays, visits, the family group's plans
- [[Learning]] (20-areas): the woodwork course and the Spanish evening class
- [[Volunteering]] (20-areas): the repair cafe and the library board
- [[Car]] (20-areas): service dates, insurance and the winter tyres
- [[Reading]] (30-resources/reading): the reading hub, by year of publication
- [[Recipes]] (30-resources/recipes): the recipe hub, by course
- [[How-to]] (30-resources/how-to): the procedures hub, by room and system
- [[People]] (30-resources/people): the people hub, by how we know them
- [[Tools]] (30-resources/how-to): what tools we own and where they live
- [[Frost Dates]] (30-resources/how-to): first and last frost for the allotment, by year
- [[Insurance]] (20-areas): policies, renewal dates and claim numbers
- [[Bike Fit]] (30-resources/how-to): saddle height and reach for each bike
- [[Paint Colours]] (30-resources/how-to): every room's paint, brand and code
- [[Boiler]] (30-resources/how-to): the boiler's model, service history and resets

## Common mistakes

Each of these has happened. Check a change against them before committing.

- Starting a note with a `# Title` heading. The file name is the title; no H1.
- Linking by path (`[[30-resources/reading/Reading]]`) instead of by name (`[[Reading]]`).
- Linking with Markdown (`[Reading](Reading.md)`); use a wikilink.
- Putting a reading note in the inbox or at the vault root instead of 30-resources/reading.
- Naming a reading note without its year, or with the year it was read instead of published.
- Leaving the front matter out, or using keys that aren't in the type's list.
- Inventing a rating the user didn't give. Leave `rating` out instead.
- Adding a tag that isn't in the list. Ask first.
- Copying long passages from a source instead of summarising.
- Writing dates as 'Nov 3' or '3/11'; it's YYYY-MM-DD.
- Leaving TODO markers in a finished note.

## Weekly review template

```markdown
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
```

## Examples

One finished note of each type, as a model to copy.

### A reading note: `Reading - The Quiet Index (2023).md`

```markdown
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
```

### A project note: `Garden Shed.md`

```markdown
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
```

### An area note: `Health.md`

```markdown
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
```

### A recipe: `Lentil soup.md`

```markdown
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
```

### A how-to: `Bleeding the radiators.md`

```markdown
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
```

### A meeting note: `Meeting - shed planning (2026-09-14).md`

```markdown
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
```

## Change log of these instructions

- 2023-01: Tags: added `energy` for the heat pump work.
- 2023-02: Naming: meeting notes take their date in brackets, not in front.
- 2023-03: Style: dates are YYYY-MM-DD in prose as well as front matter.
- 2023-04: Folders: recipes moved under 30-resources.
- 2023-05: Attachments: originals are kept uncropped.
- 2023-06: Style: currency codes for anything that isn't USD.
- 2023-07: Note types: how-to notes record when they were last checked.
- 2023-08: Tags: `volunteering` replaces `repair-cafe` and `library`.
- 2023-09: Folders: 40-journal holds weekly reviews too.
- 2023-10: Style: no bold in running text.
- 2023-11: Tags: added `energy` for the heat pump work.
- 2023-12: Naming: meeting notes take their date in brackets, not in front.
- 2024-01: Style: dates are YYYY-MM-DD in prose as well as front matter.
- 2024-02: Folders: recipes moved under 30-resources.
- 2024-03: Attachments: originals are kept uncropped.
- 2024-04: Style: currency codes for anything that isn't USD.
- 2024-05: Note types: how-to notes record when they were last checked.
- 2024-06: Tags: `volunteering` replaces `repair-cafe` and `library`.
- 2024-07: Folders: 40-journal holds weekly reviews too.
- 2024-08: Style: no bold in running text.
- 2024-09: Tags: added `energy` for the heat pump work.
- 2024-10: Naming: meeting notes take their date in brackets, not in front.
- 2024-11: Style: dates are YYYY-MM-DD in prose as well as front matter.
- 2024-12: Folders: recipes moved under 30-resources.
- 2025-01: Attachments: originals are kept uncropped.
- 2025-02: Style: currency codes for anything that isn't USD.
- 2025-03: Note types: how-to notes record when they were last checked.
- 2025-04: Tags: `volunteering` replaces `repair-cafe` and `library`.
- 2025-05: Folders: 40-journal holds weekly reviews too.
- 2025-06: Style: no bold in running text.
- 2025-07: Tags: added `energy` for the heat pump work.
- 2025-08: Naming: meeting notes take their date in brackets, not in front.
- 2025-09: Style: dates are YYYY-MM-DD in prose as well as front matter.
- 2025-10: Folders: recipes moved under 30-resources.
- 2025-11: Attachments: originals are kept uncropped.
- 2025-12: Style: currency codes for anything that isn't USD.
- 2026-01: Note types: how-to notes record when they were last checked.
- 2026-02: Tags: `volunteering` replaces `repair-cafe` and `library`.
- 2026-03: Folders: 40-journal holds weekly reviews too.
- 2026-04: Style: no bold in running text.
- 2026-05: Tags: added `energy` for the heat pump work.
- 2026-06: Naming: meeting notes take their date in brackets, not in front.
- 2026-07: Style: dates are YYYY-MM-DD in prose as well as front matter.
- 2026-08: Folders: recipes moved under 30-resources.
- 2026-09: Attachments: originals are kept uncropped.
- 2026-10: Style: currency codes for anything that isn't USD.
- 2026-11: Note types: how-to notes record when they were last checked.
- 2026-12: Tags: `volunteering` replaces `repair-cafe` and `library`.

## Hubs

Every folder of notes has a hub: a note named after the folder's subject that lists
the notes in it. Home.md links every hub. The reading hub is 30-resources/reading/Reading.md, named
'Reading'.

- **A new note goes on its hub, in the same change.** Add a link to it on the hub,
  as `[[<note name>]]`, followed by a dash and a one-line summary.
- **The reading hub is grouped by year of publication, newest year first,** under a
  `## <year>` heading per year. Add the year's heading if it isn't there yet, in its
  place in the order. Within a year, notes are in alphabetical order.
- **A new note links back to its hub** from its `## Related` section, with the hub's
  name as the link: `[[Reading]]` for a reading note.
- Don't reorder or reword other entries on a hub while you're there.

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
