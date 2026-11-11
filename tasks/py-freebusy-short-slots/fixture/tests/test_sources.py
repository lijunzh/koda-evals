import unittest
from datetime import datetime
from pathlib import Path

from freebusy.sources import bookings, calendar

DATA = Path(__file__).resolve().parent.parent / "data"


class CalendarTest(unittest.TestCase):
    def test_reads_events(self):
        events = calendar.read(DATA / "calendar.ics")
        self.assertEqual(len(events), 9)
        summary, iv = events[0]
        self.assertEqual(summary, "Standup")
        self.assertEqual(iv.start, datetime(2026, 11, 12, 9, 0))
        self.assertEqual(iv.end, datetime(2026, 11, 12, 9, 30))


class BookingsTest(unittest.TestCase):
    def test_reads_rows(self):
        rows = bookings.read(DATA / "bookings.csv")
        self.assertEqual(len(rows), 11)
        self.assertEqual((rows[0].room, rows[0].booked_by), ("Ash", "dana"))
        self.assertEqual(rows[0].interval.start, datetime(2026, 11, 12, 9, 30))


if __name__ == "__main__":
    unittest.main()
