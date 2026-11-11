"""Hidden check for assistant-draft-reply: one draft in drafts/, a reply to
Jordan's email (To, In-Reply-To as the README asks), offering at least two
specific times this weekend when Sam has two free hours, and no time that
isn't. Sam's weekend, from the calendar and the 08:00-21:00 rule:

  Saturday 11-14  busy 09-12, 13-17, 19-22  ->  two free hours only from 17:00
  Sunday   11-15  busy 10-12, 14-15         ->  from 08:00, 12:00, or 15:00-19:00

Offered times are read from the text: a clock time (`5pm`, `17:00`, `noon`,
the start of a range like `12-2pm`) belongs to the last Saturday or Sunday
named before it. Clauses that decline (`can't`, `doesn't work`, ...) or only
describe an event (`Mia has soccer from 9 to noon`) are skipped, so the decline of Saturday morning doesn't count as an offer, and
quoted text from Jordan's email is ignored.
"""

import email
import email.policy
import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JORDAN = "jordan.park@example.net"
JORDAN_ID = "<20261111.jordan.move@example.net>"
# Free stretches, in minutes from midnight: an offer at t needs [t, t+120) inside one.
FREE = {
    "saturday": [(8 * 60, 9 * 60), (12 * 60, 13 * 60), (17 * 60, 19 * 60)],
    "sunday": [(8 * 60, 10 * 60), (12 * 60, 14 * 60), (15 * 60, 21 * 60)],
}
DECLINE = re.compile(r"can'?t|can’t|cannot|can not|won'?t|won’t|unable|not able|doesn'?t work|doesn’t work|"
                     r"does not work|isn'?t going to|busy|tied up|unfortunately|no good|not free|not available", re.I)
# A clause that only describes an event ("Mia has a soccer tournament from 9 to
# noon") explains the decline; one that also offers ("I'm free", "could",
# "how about") or says nothing else, like a bullet with a time, is an offer.
EXPLAINS = re.compile(r"\b(has|have|had|got|tournament|party|brunch|lesson|dinner|game|appointment|soccer|swim|birthday|plans)\b", re.I)
OFFERS = re.compile(r"\b(free|available|open|could|can|works?|how about|would|happy to|glad to|either)\b", re.I)
DAY = re.compile(r"\b(saturday|sunday|sat|sun|monday|tuesday|wednesday|thursday|friday)\b", re.I)
TIME = re.compile(
    r"\b(?P<noon>noon|midday)\b(?:\s*(?:-|\u2013|\u2014|to|until|till)\s*\d{1,2}(?::[0-5]\d)?\s*(?:[ap]\.?m\.?\b)?)?"
    r"|\b(?P<h>\d{1,2})(?::(?P<m>[0-5]\d))?\s*(?P<ap>[ap]\.?m\.?\b)?"
    r"(?:\s*(?:-|–|—|to|until|till)\s*(?:(?P<h2>\d{1,2})(?::[0-5]\d)?\s*(?P<ap2>[ap]\.?m\.?\b)?|noon))?",
    re.I,
)


def minutes(h, m, ap, fallback_ap=None):
    h, m = int(h), int(m or 0)
    ap = (ap or fallback_ap or "").lower().replace(".", "")
    if ap == "pm" and h < 12:
        h += 12
    elif ap == "am" and h == 12:
        h = 0
    elif not ap and 1 <= h <= 7:
        h += 12  # a bare 1-7 on a weekend plan means the afternoon
    return h * 60 + m


def offers(body):
    """(day, start minute) for each clock time offered, in order."""
    day = None
    found = []
    # Rejoin lines wrapped mid-sentence; blank lines and bullets stay breaks.
    body = re.sub(r"(?<!\n)\n(?!\n|\s*[-*\u2022]\s|\s*\d+[.)]\s)", " ", body)
    clauses = []
    for sentence in re.split(r"(?<=[.!?])\s+|\n+", body):
        clauses += re.split(r"[,;]|:\s|\s\u2014\s|\bbut\b|\bhowever\b|\bthough\b", sentence, flags=re.I)
    for sentence in clauses:
        declined = bool(DECLINE.search(sentence)) or (
            bool(EXPLAINS.search(sentence)) and not OFFERS.search(sentence))
        tokens = sorted([(m.start(), "day", m) for m in DAY.finditer(sentence)]
                        + [(m.start(), "time", m) for m in TIME.finditer(sentence)], key=lambda t: t[0])
        for _, kind, m in tokens:
            if kind == "day":
                day = {"sat": "saturday", "sun": "sunday"}.get(m.group(1).lower(), m.group(1).lower())
                continue
            if m.group("noon"):
                start = 12 * 60
            else:
                clock = m.group("m") or m.group("ap") or m.group("h2")
                if not clock and not re.search(r"\b(at|from|around|after|by)\s*$", sentence[:m.start()], re.I):
                    continue  # a bare number: "two hours", "the 14th"
                if not 0 <= int(m.group("h")) <= 23:
                    continue
                start = minutes(m.group("h"), m.group("m"), m.group("ap"), m.group("ap2"))
            if not declined:
                found.append((day, start))
    return found


def body_of(msg):
    part = msg.get_body(preferencelist=("plain",)) if msg.is_multipart() else msg
    text = part.get_content() if part is not None else ""
    kept = []
    for line in text.splitlines():
        if re.match(r"\s*On .* wrote:\s*$", line):
            break
        if line.lstrip().startswith(">"):
            continue
        kept.append(line)
    return "\n".join(kept)


def inputs_unchanged(test):
    """mail/ and calendar/ are Sam's data: the task only reads them."""
    try:
        out = subprocess.run(["git", "status", "--porcelain", "--", "mail", "calendar"],
                             cwd=ROOT, capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return
    if out.returncode == 0:
        test.assertEqual(out.stdout.strip(), "", "mail/ or calendar/ was changed")


class EvalHiddenTest(unittest.TestCase):
    def draft(self):
        drafts = sorted((ROOT / "drafts").glob("*.eml"))
        self.assertEqual(len(drafts), 1, f"want one draft in drafts/: {[d.name for d in drafts]}")
        return email.message_from_bytes(drafts[0].read_bytes(), policy=email.policy.default)

    def test_eval_hidden_a_reply_to_jordan(self):
        msg = self.draft()
        self.assertIn(JORDAN, str(msg["To"] or "").lower())
        self.assertEqual(str(msg["In-Reply-To"] or "").strip(), JORDAN_ID)
        self.assertTrue(str(msg["Subject"] or "").strip(), "no Subject")

    def test_eval_hidden_offers_two_free_weekend_times(self):
        found = offers(body_of(self.draft()))
        bad = [(d, s) for d, s in found
               if d not in FREE or not any(a <= s and s + 120 <= b for a, b in FREE[d])]
        self.assertEqual(bad, [], f"offered times Sam isn't free for two hours: {bad} (all: {found})")
        self.assertGreaterEqual(len(set(found)), 2, f"want two specific times: {found}")

    def test_eval_hidden_inputs_unchanged(self):
        inputs_unchanged(self)


if __name__ == "__main__":
    unittest.main()
