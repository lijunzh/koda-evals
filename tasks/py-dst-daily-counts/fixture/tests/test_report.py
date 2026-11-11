import unittest
from datetime import date

from eventlog.parse import parse_line, read_events
from eventlog.report import daily_counts, hourly_rate, level_counts


def events(*stamps):
    return read_events(f"{s} INFO e" for s in stamps)


class ParseTest(unittest.TestCase):
    def test_parse_line(self):
        e = parse_line("2026-06-01T12:00:00Z WARN disk 91% full\n")
        self.assertEqual(e.level, "WARN")
        self.assertEqual(e.message, "disk 91% full")
        self.assertEqual(e.when.isoformat(), "2026-06-01T12:00:00+00:00")

    def test_read_events_skips_comments_and_sorts(self):
        evs = read_events(["# header\n", "2026-06-02T00:00:00Z INFO b\n", "\n",
                           "2026-06-01T00:00:00Z INFO a\n"])
        self.assertEqual([e.message for e in evs], ["a", "b"])


class ReportTest(unittest.TestCase):
    def test_utc_days(self):
        evs = events("2026-06-01T00:00:00Z", "2026-06-01T23:59:59Z", "2026-06-02T00:00:00Z")
        self.assertEqual(daily_counts(evs, "UTC"), {date(2026, 6, 1): 2, date(2026, 6, 2): 1})

    def test_new_york_summer(self):
        # 03:30 UTC on June 2 is 23:30 the evening before in New York.
        evs = events("2026-06-01T12:00:00Z", "2026-06-02T03:30:00Z", "2026-06-02T04:30:00Z")
        self.assertEqual(daily_counts(evs, "America/New_York"),
                         {date(2026, 6, 1): 2, date(2026, 6, 2): 1})

    def test_hourly_rate_on_an_ordinary_day(self):
        evs = events(*[f"2026-06-01T{h:02d}:00:00Z" for h in range(24)])
        self.assertEqual(hourly_rate(evs, "UTC"), {date(2026, 6, 1): 1.0})

    def test_level_counts(self):
        evs = read_events(["2026-06-01T10:00:00Z INFO a", "2026-06-01T11:00:00Z WARN b",
                           "2026-06-01T12:00:00Z INFO c"])
        self.assertEqual(level_counts(evs, "UTC"), {date(2026, 6, 1): {"INFO": 2, "WARN": 1}})


if __name__ == "__main__":
    unittest.main()
