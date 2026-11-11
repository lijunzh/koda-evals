"""Hidden tests: local days and their length across the clock changes."""

import unittest
from datetime import date

from eventlog.parse import read_events
from eventlog.report import daily_counts, hourly_rate, level_counts


def events(*stamps, level="INFO"):
    return read_events(f"{s} {level} e" for s in stamps)


class EvalHiddenDays(unittest.TestCase):
    def test_new_york_spring_forward(self):
        # Clocks go 02:00 -> 03:00 EST->EDT at 07:00 UTC on 2026-03-08.
        evs = events(
            "2026-03-07T17:00:00Z",  # 12:00 EST, Mar 7
            "2026-03-08T04:30:00Z",  # 23:30 EST, Mar 7
            "2026-03-08T05:30:00Z",  # 00:30 EST, Mar 8
            "2026-03-09T03:30:00Z",  # 23:30 EDT, Mar 8
            "2026-03-09T04:30:00Z",  # 00:30 EDT, Mar 9
        )
        self.assertEqual(
            daily_counts(evs, "America/New_York"),
            {date(2026, 3, 7): 2, date(2026, 3, 8): 2, date(2026, 3, 9): 1},
        )

    def test_new_york_fall_back(self):
        # Clocks go 02:00 -> 01:00 EDT->EST at 06:00 UTC on 2026-11-01.
        evs = events(
            "2026-10-31T16:00:00Z",  # 12:00 EDT, Oct 31
            "2026-11-01T03:30:00Z",  # 23:30 EDT, Oct 31
            "2026-11-01T04:30:00Z",  # 00:30 EDT, Nov 1
            "2026-11-02T04:30:00Z",  # 23:30 EST, Nov 1
            "2026-11-02T05:30:00Z",  # 00:30 EST, Nov 2
        )
        self.assertEqual(
            daily_counts(evs, "America/New_York"),
            {date(2026, 10, 31): 2, date(2026, 11, 1): 2, date(2026, 11, 2): 1},
        )

    def test_berlin_both_changes(self):
        spring = events(
            "2026-03-28T12:00:00Z",  # 13:00 CET, Mar 28
            "2026-03-29T22:30:00Z",  # 00:30 CEST, Mar 30
        )
        self.assertEqual(
            daily_counts(spring, "Europe/Berlin"),
            {date(2026, 3, 28): 1, date(2026, 3, 30): 1},
        )
        autumn = events(
            "2026-10-24T12:00:00Z",  # 14:00 CEST, Oct 24
            "2026-10-25T23:30:00Z",  # 00:30 CET, Oct 26
        )
        self.assertEqual(
            daily_counts(autumn, "Europe/Berlin"),
            {date(2026, 10, 24): 1, date(2026, 10, 26): 1},
        )

    def test_a_batch_starting_in_winter_ending_in_summer(self):
        evs = events("2026-01-15T12:00:00Z", "2026-07-02T03:30:00Z")  # 23:30 EDT, Jul 1
        self.assertEqual(
            daily_counts(evs, "America/New_York"),
            {date(2026, 1, 15): 1, date(2026, 7, 1): 1},
        )

    def test_levels_use_the_same_days(self):
        evs = read_events([
            "2026-03-07T17:00:00Z WARN a",
            "2026-03-09T03:30:00Z ERROR b",  # 23:30 EDT, Mar 8
            "2026-03-09T04:30:00Z INFO c",   # 00:30 EDT, Mar 9
        ])
        self.assertEqual(
            level_counts(evs, "America/New_York"),
            {
                date(2026, 3, 7): {"WARN": 1},
                date(2026, 3, 8): {"ERROR": 1},
                date(2026, 3, 9): {"INFO": 1},
            },
        )


class EvalHiddenRates(unittest.TestCase):
    def test_short_and_long_days(self):
        # 23 events on the 23-hour day, 25 on the 25-hour one: one an hour.
        spring = events(*[f"2026-03-08T{5 + h:02d}:10:00Z" for h in range(19)],
                        *[f"2026-03-09T{h:02d}:10:00Z" for h in range(4)])
        rates = hourly_rate(spring, "America/New_York")
        self.assertEqual(daily_counts(spring, "America/New_York"), {date(2026, 3, 8): 23})
        self.assertAlmostEqual(rates[date(2026, 3, 8)], 1.0)

        autumn = events(*[f"2026-11-01T{4 + h:02d}:10:00Z" for h in range(20)],
                        *[f"2026-11-02T{h:02d}:10:00Z" for h in range(5)])
        rates = hourly_rate(autumn, "America/New_York")
        self.assertEqual(daily_counts(autumn, "America/New_York"), {date(2026, 11, 1): 25})
        self.assertAlmostEqual(rates[date(2026, 11, 1)], 1.0)

    def test_an_ordinary_day_is_24_hours(self):
        evs = events(*[f"2026-06-01T{4 + h:02d}:00:00Z" for h in range(20)],
                     *[f"2026-06-02T{h:02d}:00:00Z" for h in range(4)])
        self.assertAlmostEqual(hourly_rate(evs, "America/New_York")[date(2026, 6, 1)], 1.0)


if __name__ == "__main__":
    unittest.main()
