//! ⌘+, ⌘- and ⌘0: every pane's font a step larger or smaller than Settings' size, for this run
//! only (P5-143e). Settings keeps its value; ⌘0 or opening paddock again goes back to it.

/// The smallest and largest sizes the steps go to, in points.
pub const SMALLEST: f32 = 8.0;
pub const LARGEST: f32 = 40.0;
/// How much one step changes the size, in points.
pub const STEP: f32 = 1.0;

/// The font size `steps` steps from Settings' `base`, within the limits; a `base` outside them
/// stays as it is until a step brings it in.
pub fn size(base: f32, steps: i32) -> f32 {
    if steps == 0 {
        return base;
    }
    (base + steps as f32 * STEP).clamp(SMALLEST.min(base), LARGEST.max(base))
}

/// The steps after one more `by` (1 larger, -1 smaller): none past the limits, so a step back
/// from a limit always shows.
pub fn step(base: f32, steps: i32, by: i32) -> i32 {
    let next = steps + by;
    if size(base, next) == size(base, steps) {
        steps
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_at_a_time_from_settings_size() {
        assert_eq!(size(14.0, 0), 14.0);
        assert_eq!(size(14.0, 2), 16.0);
        assert_eq!(size(14.5, -3), 11.5);
        assert_eq!(step(14.0, 0, 1), 1);
        assert_eq!(step(14.0, 1, -1), 0);
    }

    #[test]
    fn not_past_the_limits_and_back_at_once() {
        // Up to the largest, then no further: one step down shows straight away.
        let mut steps = 0;
        for _ in 0..100 {
            steps = step(14.0, steps, 1);
        }
        assert_eq!(size(14.0, steps), LARGEST);
        assert!(size(14.0, step(14.0, steps, -1)) < LARGEST);
        let mut steps = 0;
        for _ in 0..100 {
            steps = step(14.5, steps, -1);
        }
        assert_eq!(size(14.5, steps), SMALLEST);
        assert!(size(14.5, step(14.5, steps, 1)) > SMALLEST);
    }

    #[test]
    fn a_settings_size_outside_the_limits_is_kept_until_stepped() {
        assert_eq!(size(6.0, 0), 6.0);
        assert_eq!(step(6.0, 0, -1), 0);
        assert!(size(6.0, step(6.0, 0, 1)) > 6.0);
        assert_eq!(size(48.0, 0), 48.0);
        assert_eq!(step(48.0, 0, 1), 0);
    }
}
