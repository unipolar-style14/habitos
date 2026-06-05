//! GitHub-contributions-style heatmap of habit completion over time.
//! Pure computation — no DB, no terminal output. Caller renders.

use std::collections::HashMap;
use time::Date;

/// 5 intensity levels: 0 (none) → 4 (best).
pub const INTENSITIES: usize = 5;

#[derive(Debug, Clone)]
pub struct Heatmap {
    /// Grid is `weeks × 7`. `cells[w][d]` is intensity at week w, weekday d
    /// (0 = Monday). Cells outside the requested window are 0.
    pub cells: Vec<[u8; 7]>,
    /// First date represented in the grid (the Monday on or before `start`).
    pub aligned_start: Date,
    /// Today (the end of the window).
    pub end: Date,
    /// First date inside the requested window (`end - days_back`).
    pub window_start: Date,
}

/// Build a heatmap from per-day done-habit counts.
///
/// `done_per_day` maps date → number of habits marked done that day.
/// `days_back` is how many days before `today` to include (inclusive of today).
pub fn build(today: Date, days_back: u32, done_per_day: &HashMap<Date, u32>) -> Heatmap {
    let window_start = today - time::Duration::days(days_back as i64);

    // Align grid start to the Monday on/before window_start.
    let weekday_offset = window_start.weekday().number_days_from_monday() as i64;
    let aligned_start = window_start - time::Duration::days(weekday_offset);

    let total_days = (today - aligned_start).whole_days() as usize + 1;
    let weeks = total_days.div_ceil(7);
    let mut cells = vec![[0u8; 7]; weeks];

    let mut cursor = aligned_start;
    for week in cells.iter_mut() {
        for cell in week.iter_mut() {
            if cursor > today {
                break;
            }
            if cursor >= window_start {
                let count = done_per_day.get(&cursor).copied().unwrap_or(0);
                *cell = intensity(count);
            }
            match cursor.next_day() {
                Some(d) => cursor = d,
                None => break,
            }
        }
    }

    Heatmap {
        cells,
        aligned_start,
        end: today,
        window_start,
    }
}

fn intensity(count: u32) -> u8 {
    match count {
        0 => 0,
        1 => 1,
        2 => 2,
        3..=4 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn empty_heatmap_is_all_zeros() {
        let h = build(date!(2026 - 06 - 05), 30, &HashMap::new());
        assert!(h.cells.iter().all(|w| w.iter().all(|&c| c == 0)));
    }

    #[test]
    fn intensity_scales_with_count() {
        let mut m = HashMap::new();
        m.insert(date!(2026 - 06 - 05), 1);
        m.insert(date!(2026 - 06 - 04), 5);
        let h = build(date!(2026 - 06 - 05), 7, &m);
        let total_max: u8 = h.cells.iter().flatten().copied().max().unwrap();
        assert_eq!(total_max, 4, "5 done = max intensity");
    }

    #[test]
    fn grid_aligned_to_monday() {
        // 2026-06-05 is a Friday. Aligned start of a 7-day window:
        // window_start = 2026-05-29 (Friday). Aligned Monday = 2026-05-25.
        let h = build(date!(2026 - 06 - 05), 7, &HashMap::new());
        assert_eq!(h.aligned_start.weekday(), time::Weekday::Monday);
    }
}
