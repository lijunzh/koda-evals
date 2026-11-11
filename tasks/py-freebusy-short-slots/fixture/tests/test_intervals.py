import unittest
from datetime import datetime

from freebusy.intervals import Interval, busy_minutes, free_slots, merge


def at(hhmm: str) -> datetime:
    h, m = map(int, hhmm.split(":"))
    return datetime(2026, 11, 12, h, m)


def iv(a: str, b: str) -> Interval:
    return Interval(at(a), at(b))


DAY = iv("09:00", "17:00")


class MergeTest(unittest.TestCase):
    def test_touching_and_overlapping_combine(self):
        got = merge([iv("10:00", "11:00"), iv("09:00", "10:00"), iv("10:30", "10:45")])
        self.assertEqual(got, [iv("09:00", "11:00")])

    def test_gaps_stay(self):
        got = merge([iv("09:00", "09:30"), iv("09:45", "10:00")])
        self.assertEqual(got, [iv("09:00", "09:30"), iv("09:45", "10:00")])


class FreeSlotsTest(unittest.TestCase):
    def test_gaps_between_meetings(self):
        got = free_slots([iv("09:00", "10:00"), iv("12:00", "13:00")], DAY)
        self.assertEqual(got, [iv("10:00", "12:00"), iv("13:00", "17:00")])

    def test_min_minutes_hides_short_slots(self):
        got = free_slots([iv("09:00", "10:00"), iv("10:10", "17:00")], DAY, min_minutes=15)
        self.assertEqual(got, [])

    def test_busy_minutes_counts_overlap_once(self):
        self.assertEqual(busy_minutes([iv("09:00", "10:00"), iv("09:30", "10:30")], DAY), 90)


if __name__ == "__main__":
    unittest.main()
