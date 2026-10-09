use crate::{Error, Result};
use ichnos_core::chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Timelike};
use std::collections::BTreeSet;

/// Weekly wall-clock business schedule, measured in seconds from Monday midnight.
#[derive(Clone, Debug)]
pub struct BusinessHours {
    /// Working intervals, between zero and 604800 seconds; overlaps are unified.
    pub slots: Vec<(u32, u32)>,
    /// Dates excluded from the working calendar.
    pub non_working_dates: BTreeSet<NaiveDate>,
}
impl Default for BusinessHours {
    fn default() -> Self {
        Self {
            slots: (0..5)
                .map(|d| (d * 86400 + 7 * 3600, d * 86400 + 17 * 3600))
                .collect(),
            non_working_dates: BTreeSet::new(),
        }
    }
}
impl BusinessHours {
    /// Scheduled wall-clock seconds between two instants, stripping their offsets as pm4py does.
    pub fn seconds_between(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Result<f64> {
        let start = start.naive_local();
        let end = end.naive_local();
        if self.slots.iter().any(|(a, b)| a > b || *b > 604800) {
            return Err(Error::InvalidOption(
                "business slots must lie within a week",
            ));
        }
        if end <= start {
            return Ok(0.0);
        }
        let mut slots = self.slots.clone();
        slots.sort_unstable();
        let mut unified: Vec<(u32, u32)> = Vec::new();
        for (a, b) in slots {
            if let Some(last) = unified
                .last_mut()
                .filter(|last| last.1.saturating_add(1) >= a)
            {
                last.1 = last.1.max(b);
            } else {
                unified.push((a, b));
            }
        }
        let week_second = |d: ichnos_core::chrono::NaiveDateTime| {
            d.weekday().num_days_from_monday() as f64 * 86400.0
                + d.num_seconds_from_midnight() as f64
                + d.nanosecond() as f64 / 1e9
        };
        let cumulative = |d| {
            let s = week_second(d);
            unified
                .iter()
                .map(|&(a, b)| (s.min(b as f64) - a as f64).max(0.0))
                .sum::<f64>()
        };
        let monday = |d: ichnos_core::chrono::NaiveDateTime| {
            d.date().num_days_from_ce() - d.weekday().num_days_from_monday() as i32
        };
        let weeks = (monday(end) - monday(start)) / 7;
        let mut total = weeks as f64 * unified.iter().map(|(a, b)| (b - a) as f64).sum::<f64>()
            + cumulative(end)
            - cumulative(start);
        for day in self.non_working_dates.range(start.date()..=end.date()) {
            let lower = if *day == start.date() {
                start.num_seconds_from_midnight() as f64 + start.nanosecond() as f64 / 1e9
            } else {
                0.0
            };
            let upper = if *day == end.date() {
                end.num_seconds_from_midnight() as f64 + end.nanosecond() as f64 / 1e9
            } else {
                86400.0
            };
            let offset = day.weekday().num_days_from_monday() as f64 * 86400.0;
            // Subtract each scheduled intersection on this excluded date.
            for &(a, b) in &unified {
                total -= ((offset + upper).min(b as f64) - (offset + lower).max(a as f64)).max(0.0);
            }
        }
        Ok(total)
    }
}
