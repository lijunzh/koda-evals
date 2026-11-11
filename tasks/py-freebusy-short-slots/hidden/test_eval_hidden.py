"""Hidden tests for py-freebusy-short-slots.

The 1-minute slots come from the booking export, whose `end` is the last
booked minute (data/README.md): a booking written 09:30,09:59 runs until
10:00. Hiding short slots in the free-time view clears the symptom but
leaves every booking a minute short in busy time and utilization, and
would also hide real one-minute gaps.
"""

import tempfile
import unittest
from datetime import date, datetime
from pathlib import Path

from freebusy.intervals import Interval
from freebusy.report import day_report

DATA = Path(__file__).resolve().parent.parent / "data"


def at(day: int, hhmm: str) -> datetime:
    h, m = map(int, hhmm.split(":"))
    return datetime(2026, 11, day, h, m)


def iv(day: int, a: str, b: str) -> Interval:
    return Interval(at(day, a), at(day, b))


def write_data(folder: Path, events, booking_rows) -> None:
    lines = ["BEGIN:VCALENDAR", "VERSION:2.0"]
    for i, (start, end) in enumerate(events):
        lines += ["BEGIN:VEVENT", f"UID:e{i}", f"DTSTART:{start}", f"DTEND:{end}", f"SUMMARY:e{i}", "END:VEVENT"]
    lines.append("END:VCALENDAR")
    (folder / "calendar.ics").write_text("\r\n".join(lines) + "\r\n", encoding="utf-8")
    rows = ["room,booked_by,date,start,end"] + [",".join(r) for r in booking_rows]
    (folder / "bookings.csv").write_text("\n".join(rows) + "\n", encoding="utf-8")


class EvalHiddenTest(unittest.TestCase):
    def test_eval_hidden_a_booking_runs_through_its_last_minute(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_data(Path(tmp), [], [("Ash", "dana", "2026-11-12", "09:00", "09:59"),
                                       ("Ash", "dana", "2026-11-12", "16:00", "16:59")])
            d = day_report(Path(tmp), "dana", date(2026, 11, 12))
        self.assertEqual(d.free, [iv(12, "10:00", "16:00")])
        self.assertAlmostEqual(d.busy_minutes, 120)
        self.assertAlmostEqual(d.utilization, 0.25)

    def test_eval_hidden_dana_on_the_12th(self):
        d = day_report(DATA, "dana", date(2026, 11, 12))
        self.assertEqual(d.free, [iv(12, "12:00", "13:00"), iv(12, "13:45", "14:00"),
                                  iv(12, "15:00", "15:30"), iv(12, "16:00", "17:00")])
        self.assertAlmostEqual(d.busy_minutes, 315)
        self.assertAlmostEqual(d.utilization, 315 / 480)

    def test_eval_hidden_dana_on_the_13th(self):
        d = day_report(DATA, "dana", date(2026, 11, 13))
        self.assertEqual(d.free, [iv(13, "12:00", "14:00"), iv(13, "15:30", "17:00")])
        self.assertAlmostEqual(d.busy_minutes, 270)

    def test_eval_hidden_a_real_one_minute_gap_is_still_free(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_data(Path(tmp),
                       [("20261112T090000", "20261112T100000"), ("20261112T110100", "20261112T120000")],
                       [("Ash", "dana", "2026-11-12", "10:01", "10:59")])
            d = day_report(Path(tmp), "dana", date(2026, 11, 12))
        self.assertEqual(d.free, [iv(12, "10:00", "10:01"), iv(12, "11:00", "11:01"), iv(12, "12:00", "17:00")])
        self.assertAlmostEqual(d.busy_minutes, 60 + 59 + 59)

    def test_eval_hidden_min_minutes_still_filters(self):
        d = day_report(DATA, "dana", date(2026, 11, 12), min_minutes=30)
        self.assertEqual(d.free, [iv(12, "12:00", "13:00"), iv(12, "15:00", "15:30"), iv(12, "16:00", "17:00")])


if __name__ == "__main__":
    unittest.main()
