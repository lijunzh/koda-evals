//! Weekday helpers for the shift scheduler.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// The seven days starting at Monday.
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// The day after this one, wrapping from Sunday to Monday.
    pub fn next(self) -> Weekday {
        let i = Weekday::ALL.iter().position(|d| *d == self).unwrap();
        Weekday::ALL[(i + 1) % 7]
    }
}

/// True on Saturday and Sunday; shifts on those days pay the weekend rate.
pub fn is_weekend(day: Weekday) -> bool {
    matches!(day, Weekday::Friday | Weekday::Saturday | Weekday::Sunday)
}

/// Number of weekend days in the `n` days starting at `from`, inclusive.
pub fn weekend_days(from: Weekday, n: usize) -> usize {
    let mut day = from;
    let mut count = 0;
    for _ in 0..n {
        if is_weekend(day) {
            count += 1;
        }
        day = day.next();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps() {
        assert_eq!(Weekday::Sunday.next(), Weekday::Monday);
    }

    #[test]
    fn a_full_week_has_two_weekend_days_on_saturday_and_sunday() {
        assert!(is_weekend(Weekday::Saturday));
        assert!(is_weekend(Weekday::Sunday));
    }
}
