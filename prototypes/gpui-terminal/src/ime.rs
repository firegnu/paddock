//! The input method's in-progress (marked) text. Ranges are UTF-16 offsets, as the platform
//! input handler reports them; the terminal only ever receives committed text.
use std::ops::Range;

#[derive(Debug, Default)]
pub struct Composition {
    marked: Option<String>,
}
impl Composition {
    pub fn mark(&mut self, text: &str) {
        self.marked = (!text.is_empty()).then(|| text.to_owned());
    }
    /// Ends any composition and returns the bytes to write to the terminal.
    pub fn commit(&mut self, text: &str) -> Vec<u8> {
        self.marked = None;
        text.as_bytes().to_vec()
    }
    pub fn clear(&mut self) {
        self.marked = None;
    }
    pub fn marked(&self) -> Option<&str> {
        self.marked.as_deref()
    }
    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|text| 0..text.encode_utf16().count())
    }
    /// The caret sits at the end of the marked text.
    pub fn selected_range(&self) -> Range<usize> {
        let end = self.marked_range().map_or(0, |range| range.end);
        end..end
    }
    pub fn text_for_range(&self, range: Range<usize>) -> Option<String> {
        let units: Vec<u16> = self.marked.as_ref()?.encode_utf16().collect();
        let end = range.end.min(units.len());
        let start = range.start.min(end);
        String::from_utf16(&units[start..end]).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marking_tracks_utf16_length() {
        let mut ime = Composition::default();
        assert_eq!(ime.marked_range(), None);
        assert_eq!(ime.selected_range(), 0..0);
        ime.mark("ni");
        assert_eq!(ime.marked_range(), Some(0..2));
        ime.mark("你");
        assert_eq!(ime.marked(), Some("你"));
        assert_eq!(ime.marked_range(), Some(0..1));
        assert_eq!(ime.selected_range(), 1..1);
        ime.mark("😀");
        assert_eq!(ime.marked_range(), Some(0..2));
    }

    #[test]
    fn empty_marking_ends_the_composition() {
        let mut ime = Composition::default();
        ime.mark("n");
        ime.mark("");
        assert_eq!(ime.marked(), None);
    }

    #[test]
    fn commit_sends_utf8_and_clears() {
        let mut ime = Composition::default();
        ime.mark("nihao");
        assert_eq!(ime.commit("你好"), "你好".as_bytes());
        assert_eq!(ime.marked(), None);
    }

    #[test]
    fn text_for_range_reads_utf16_slices() {
        let mut ime = Composition::default();
        ime.mark("你好");
        assert_eq!(ime.text_for_range(1..2).as_deref(), Some("好"));
        assert_eq!(ime.text_for_range(0..9).as_deref(), Some("你好"));
        ime.clear();
        assert_eq!(ime.text_for_range(0..1), None);
    }
}
