//! Month grids for the calendar of daily notes.
//!
//! Daily notes are named after their date (`YYYY-MM-DD`, see
//! [`crate::cave::Cave::open_daily_note_for_date`]); the calendar shows one
//! month at a time as ISO weeks, Monday first, with the week number in front
//! of each row. The UI draws a [`Month`]'s [`Month::weeks`] and asks the cave
//! which of those days have a note ([`crate::cave::Cave::daily_note_slugs`]).

use chrono::{Datelike, Duration, Local, NaiveDate};

/// The weekday column headings, Monday first.
pub const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// The slug of today's daily note (`YYYY-MM-DD`, local time).
pub fn today_slug() -> String {
    day_slug(Local::now().date_naive())
}

/// The daily-note slug of `date`.
fn day_slug(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// A calendar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month {
    /// The first day of the month.
    first: NaiveDate,
}

/// One row of a month grid: an ISO week.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Week {
    /// The ISO week number.
    pub number: u32,
    /// Monday to Sunday.
    pub days: Vec<Day>,
}

/// One cell of a month grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Day {
    /// The day's daily-note slug, `YYYY-MM-DD`.
    pub slug: String,
    /// The day of the month, 1-based.
    pub day: u32,
    /// Whether the day belongs to the month shown (the first and last weeks
    /// run into the neighbouring months).
    pub in_month: bool,
}

impl Month {
    /// The month `month` (1–12) of `year`; `None` for an invalid month.
    pub fn new(year: i32, month: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, 1).map(|first| Self { first })
    }

    /// The month containing today (local time).
    pub fn current() -> Self {
        Self::containing(Local::now().date_naive())
    }

    /// The month of a daily-note slug (`YYYY-MM-DD`); `None` for any other
    /// name.
    pub fn of_slug(slug: &str) -> Option<Self> {
        NaiveDate::parse_from_str(slug, "%Y-%m-%d")
            .ok()
            .map(Self::containing)
    }

    fn containing(date: NaiveDate) -> Self {
        Self {
            first: date.with_day(1).unwrap_or(date),
        }
    }

    pub fn year(&self) -> i32 {
        self.first.year()
    }

    /// The month, 1–12.
    pub fn month(&self) -> u32 {
        self.first.month()
    }

    /// The month after this one.
    pub fn next(self) -> Self {
        Self::containing(self.last_day() + Duration::days(1))
    }

    /// The month before this one.
    pub fn prev(self) -> Self {
        Self::containing(self.first - Duration::days(1))
    }

    /// The heading shown above the grid, e.g. `November 2026`.
    pub fn title(&self) -> String {
        self.first.format("%B %Y").to_string()
    }

    fn last_day(&self) -> NaiveDate {
        let next_first = if self.first.month() == 12 {
            NaiveDate::from_ymd_opt(self.first.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(self.first.year(), self.first.month() + 1, 1)
        };
        next_first.map_or(self.first, |next| next - Duration::days(1))
    }

    /// The weeks that touch this month, Monday first.
    pub fn weeks(&self) -> Vec<Week> {
        let last = self.last_day();
        let mut monday =
            self.first - Duration::days(i64::from(self.first.weekday().num_days_from_monday()));
        let mut weeks = Vec::new();
        while monday <= last {
            let days = (0..7)
                .map(|offset| {
                    let date = monday + Duration::days(offset);
                    Day {
                        slug: day_slug(date),
                        day: date.day(),
                        in_month: date.month() == self.first.month(),
                    }
                })
                .collect();
            weeks.push(Week {
                number: monday.iso_week().week(),
                days,
            });
            monday += Duration::days(7);
        }
        weeks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_month_is_laid_out_in_iso_weeks_monday_first() {
        // November 2026 starts on a Sunday and ends on a Monday.
        let month = Month::new(2026, 11).unwrap();
        assert_eq!(month.title(), "November 2026");
        let weeks = month.weeks();
        assert_eq!(weeks.len(), 6);
        assert_eq!(weeks[0].number, 44);
        assert_eq!(weeks[0].days[0].slug, "2026-10-26");
        assert!(!weeks[0].days[0].in_month);
        assert_eq!(weeks[0].days[6].slug, "2026-11-01");
        assert!(weeks[0].days[6].in_month);
        assert_eq!(weeks[5].days[0].slug, "2026-11-30");
        assert_eq!(weeks[5].days[0].day, 30);
        assert!(!weeks[5].days[1].in_month);
        assert!(weeks.iter().all(|week| week.days.len() == 7));
    }

    #[test]
    fn months_step_across_years() {
        let december = Month::new(2026, 12).unwrap();
        assert_eq!(december.next(), Month::new(2027, 1).unwrap());
        assert_eq!(Month::new(2027, 1).unwrap().prev(), december);
        assert_eq!(Month::new(2026, 3).unwrap().prev().month(), 2);
        assert_eq!(Month::new(2026, 13), None);
    }

    #[test]
    fn the_week_number_of_a_year_boundary_week_is_iso() {
        // 2027-01-01 is a Friday in ISO week 53 of 2026.
        let weeks = Month::new(2027, 1).unwrap().weeks();
        assert_eq!(weeks[0].number, 53);
        assert_eq!(weeks[0].days[0].slug, "2026-12-28");
    }

    #[test]
    fn a_daily_note_slug_names_its_month() {
        assert_eq!(
            Month::of_slug("2026-02-14"),
            Some(Month::new(2026, 2).unwrap())
        );
        assert_eq!(Month::of_slug("groceries"), None);
        assert_eq!(Month::of_slug(&today_slug()), Some(Month::current()));
    }
}
