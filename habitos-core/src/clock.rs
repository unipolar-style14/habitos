use time::{Date, OffsetDateTime, UtcOffset};

/// Abstraction over "what date does the user perceive *now* to be?"
///
/// All habit/journal/review semantics are bound to the user's *local calendar
/// day*, not a UTC instant: a workout logged at 2am still counts for "today"
/// as the user thinks of it. Implementations let tests pin a fixed date.
pub trait Clock: Send + Sync {
    fn today_local(&self) -> Date;
    fn now_utc(&self) -> OffsetDateTime;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today_local(&self) -> Date {
        let now = OffsetDateTime::now_utc();
        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        now.to_offset(offset).date()
    }

    fn now_utc(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    pub today: Date,
}

#[cfg(test)]
impl Clock for FixedClock {
    fn today_local(&self) -> Date {
        self.today
    }
    fn now_utc(&self) -> OffsetDateTime {
        self.today.midnight().assume_utc()
    }
}
