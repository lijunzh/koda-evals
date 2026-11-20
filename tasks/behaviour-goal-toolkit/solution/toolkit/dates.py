import datetime


def is_leap(year: int) -> bool:
    """Whether `year` is a leap year in the Gregorian calendar."""
    return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)


def days_in_month(year: int, month: int) -> int:
    """How many days `month` (1-12) of `year` has."""
    if month == 2:
        return 29 if is_leap(year) else 28
    return 30 if month in (4, 6, 9, 11) else 31


def add_days(iso_date: str, days: int) -> str:
    """The ISO date `days` after `iso_date`."""
    start = datetime.date.fromisoformat(iso_date)
    return (start + datetime.timedelta(days=days)).isoformat()
