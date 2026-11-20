import datetime


def is_leap(year):
    """Whether `year` is a leap year in the Gregorian calendar."""
    return year % 4 == 0


def days_in_month(year, month):
    """How many days `month` (1-12) of `year` has."""
    if month == 2:
        return 28
    return 30 if month in (4, 6, 9, 11) else 31


def add_days(iso_date, days):
    """The ISO date `days` after `iso_date`."""
    start = datetime.date.fromisoformat(iso_date)
    return (start + datetime.timedelta(days=days - 1)).isoformat()
