//! The history's position bar (P5-143e): a thin bar at the pane's right edge while the view is
//! scrolled back, its thumb as long as the share of the history and screen shown and as far down
//! as the view is. Only shows where the view is; it is not dragged.

/// The shortest the thumb gets, in pixels.
pub const SHORTEST: f32 = 24.0;

/// The thumb's top and length along a track `height` tall, the view `back` lines (part of one
/// too) up from the bottom, `history` lines kept above the `rows` on the screen; none at the
/// bottom or without a history.
pub fn thumb(back: f32, history: usize, rows: u16, height: f32) -> Option<(f32, f32)> {
    if back <= 0.0 || history == 0 {
        return None;
    }
    let total = history as f32 + f32::from(rows);
    let length = (height * f32::from(rows) / total).max(SHORTEST).min(height);
    let top = (history as f32 - back.min(history as f32)) / history as f32 * (height - length);
    Some((top, length))
}

/// The grid's size shown for a moment as the pane is resized (P5-143e), as `120×40` (columns by
/// rows); none on the pane's first layout (`before` none) or when the size stayed.
pub fn size_label(before: Option<(u16, u16)>, now: (u16, u16)) -> Option<String> {
    let before = before?;
    (before != now).then(|| format!("{}×{}", now.0, now.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_at_the_bottom_or_without_a_history() {
        assert_eq!(thumb(0.0, 500, 40, 800.0), None);
        assert_eq!(thumb(3.0, 0, 40, 800.0), None);
    }

    #[test]
    fn the_thumb_is_as_long_as_the_share_shown_and_where_the_view_is() {
        // 40 rows of 200 lines: a fifth of the track.
        let (top, length) = thumb(160.0, 160, 40, 800.0).unwrap();
        assert_eq!((top, length), (0.0, 160.0));
        // Half way back.
        let (top, _) = thumb(80.0, 160, 40, 800.0).unwrap();
        assert_eq!(top, 320.0);
        // Nearly at the bottom: nearly at the track's end.
        let (top, length) = thumb(0.5, 160, 40, 800.0).unwrap();
        assert!((top + length - 800.0).abs() < 3.0, "{top} + {length}");
    }

    #[test]
    fn a_long_history_still_has_a_thumb_to_see() {
        let (_, length) = thumb(5000.0, 10_000, 40, 800.0).unwrap();
        assert_eq!(length, SHORTEST);
        // A track shorter than that: the whole track.
        let (_, length) = thumb(5.0, 10_000, 2, 20.0).unwrap();
        assert_eq!(length, 20.0);
    }

    #[test]
    fn the_size_shows_as_the_pane_is_resized_not_as_it_first_opens() {
        assert_eq!(size_label(None, (120, 40)), None);
        assert_eq!(size_label(Some((120, 40)), (120, 40)), None);
        assert_eq!(
            size_label(Some((120, 40)), (100, 40)).as_deref(),
            Some("100×40")
        );
        assert_eq!(
            size_label(Some((80, 24)), (80, 30)).as_deref(),
            Some("80×30")
        );
    }
}
