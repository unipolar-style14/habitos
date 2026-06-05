//! Auto-detected celebration moments.
//!
//! Pure functions: caller hands in the relevant stat, gets back a structured
//! milestone (or None). The CLI / TUI is responsible for actually rendering
//! the celebration line.

/// Thresholds we celebrate on each habit streak.
pub const STREAK_THRESHOLDS: [u32; 5] = [7, 30, 60, 100, 365];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Milestone {
    StreakReached { days: u32 },
}

impl Milestone {
    pub fn celebration_line(&self, habit_name: &str) -> String {
        match self {
            Milestone::StreakReached { days } => {
                let badge = match days {
                    7 => "🌱",
                    30 => "🔥",
                    60 => "⚡",
                    100 => "💯",
                    365 => "🏆",
                    _ => "✨",
                };
                format!("{badge} {days}-day streak on {habit_name} — keep going.")
            }
        }
    }
}

/// Returns Some(milestone) if `current_streak` exactly equals one of the
/// thresholds. Caller passes the *just-logged* streak; we only fire on the
/// edge transition.
pub fn streak_milestone(current_streak: u32) -> Option<Milestone> {
    STREAK_THRESHOLDS
        .iter()
        .copied()
        .find(|&t| current_streak == t)
        .map(|days| Milestone::StreakReached { days })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_on_threshold() {
        assert!(streak_milestone(7).is_some());
        assert!(streak_milestone(30).is_some());
        assert!(streak_milestone(100).is_some());
    }

    #[test]
    fn ignores_non_thresholds() {
        assert!(streak_milestone(0).is_none());
        assert!(streak_milestone(6).is_none());
        assert!(streak_milestone(8).is_none());
        assert!(streak_milestone(31).is_none());
    }

    #[test]
    fn celebration_includes_habit_name() {
        let m = Milestone::StreakReached { days: 7 };
        let line = m.celebration_line("DSA");
        assert!(line.contains("DSA"));
        assert!(line.contains("7"));
    }
}
