//! Tracking of trailing whitespace for the document renderer.

use helix_core::doc_formatter::FormattedGrapheme;
use helix_core::graphemes::Grapheme;
use helix_core::RopeSlice;
use helix_stdx::rope::RopeSliceExt;

/// Determines whether whitespace graphemes are trailing, i.e. followed by
/// nothing but whitespace up to the end of their document line.
///
/// The start of each line's trailing whitespace is computed lazily and
/// cached, so lines are only scanned once while rendering the visible area.
#[derive(Debug)]
pub struct TrailingWhitespaceTracker<'a> {
    enabled: bool,
    text: RopeSlice<'a>,
    /// The last document line whose trailing whitespace start was computed.
    current_line: usize,
    /// The char index at which the trailing whitespace of `current_line` begins.
    trailing_start: usize,
}

impl<'a> TrailingWhitespaceTracker<'a> {
    pub fn new(enabled: bool, text: RopeSlice<'a>) -> Self {
        Self {
            enabled,
            text,
            current_line: usize::MAX,
            trailing_start: 0,
        }
    }

    /// Returns whether `grapheme` is trailing whitespace.
    ///
    /// Returns `false` for virtual text, the EOF grapheme and newlines, which
    /// are never considered trailing whitespace.
    pub fn is_trailing(&mut self, grapheme: &FormattedGrapheme<'_>) -> bool {
        if !self.enabled
            || grapheme.is_virtual()
            || grapheme.source.is_eof()
            || matches!(grapheme.raw, Grapheme::Newline)
        {
            return false;
        }

        if grapheme.line_idx != self.current_line {
            self.current_line = grapheme.line_idx;
            let line_start = self.text.line_to_char(grapheme.line_idx);
            self.trailing_start = self
                .text
                .line(grapheme.line_idx)
                .last_non_whitespace_char()
                .map_or(line_start, |idx| line_start + idx + 1);
        }

        grapheme.char_idx >= self.trailing_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use helix_core::doc_formatter::GraphemeSource;
    use helix_core::{Position, Rope};

    fn doc_grapheme(
        char_idx: usize,
        line_idx: usize,
        g: &'static str,
    ) -> FormattedGrapheme<'static> {
        FormattedGrapheme {
            raw: Grapheme::Other { g: g.into() },
            source: GraphemeSource::Document { codepoints: 1 },
            visual_pos: Position { row: 0, col: 0 },
            line_idx,
            char_idx,
        }
    }

    fn eof_grapheme(char_idx: usize, line_idx: usize) -> FormattedGrapheme<'static> {
        FormattedGrapheme {
            raw: Grapheme::Other { g: " ".into() },
            source: GraphemeSource::Document { codepoints: 0 },
            visual_pos: Position { row: 0, col: 0 },
            line_idx,
            char_idx,
        }
    }

    fn newline_grapheme(char_idx: usize, line_idx: usize) -> FormattedGrapheme<'static> {
        FormattedGrapheme {
            raw: Grapheme::Newline,
            source: GraphemeSource::Document { codepoints: 1 },
            visual_pos: Position { row: 0, col: 0 },
            line_idx,
            char_idx,
        }
    }

    #[test]
    fn test_trailing_whitespace_detection() {
        // "abc  \nfoo\t\t\n"
        let text = Rope::from_str("abc  \nfoo\t\t\n");
        let mut sut = TrailingWhitespaceTracker::new(true, text.slice(..));

        // line 0: "abc  "
        assert!(!sut.is_trailing(&doc_grapheme(0, 0, "a")));
        assert!(!sut.is_trailing(&doc_grapheme(1, 0, "b")));
        assert!(!sut.is_trailing(&doc_grapheme(2, 0, "c")));
        assert!(sut.is_trailing(&doc_grapheme(3, 0, " ")));
        assert!(sut.is_trailing(&doc_grapheme(4, 0, " ")));

        // line 1: "foo\t\t"
        assert!(!sut.is_trailing(&doc_grapheme(6, 1, "f")));
        assert!(!sut.is_trailing(&doc_grapheme(8, 1, "o")));
        assert!(sut.is_trailing(&doc_grapheme(9, 1, "\t")));
        assert!(sut.is_trailing(&doc_grapheme(10, 1, "\t")));

        // the newline itself is never trailing
        assert!(!sut.is_trailing(&newline_grapheme(5, 0)));
        assert!(!sut.is_trailing(&newline_grapheme(11, 1)));

        // the EOF grapheme is never trailing
        assert!(!sut.is_trailing(&eof_grapheme(12, 2)));
    }

    #[test]
    fn test_all_whitespace_line_is_trailing() {
        // "   \nabc"
        let text = Rope::from_str("   \nabc");
        let mut sut = TrailingWhitespaceTracker::new(true, text.slice(..));

        assert!(sut.is_trailing(&doc_grapheme(0, 0, " ")));
        assert!(sut.is_trailing(&doc_grapheme(2, 0, " ")));

        assert!(!sut.is_trailing(&doc_grapheme(4, 1, "a")));
        assert!(!sut.is_trailing(&doc_grapheme(6, 1, "c")));
    }

    #[test]
    fn test_last_line_without_newline() {
        // "abc  "
        let text = Rope::from_str("abc  ");
        let mut sut = TrailingWhitespaceTracker::new(true, text.slice(..));

        assert!(!sut.is_trailing(&doc_grapheme(0, 0, "a")));
        assert!(sut.is_trailing(&doc_grapheme(3, 0, " ")));
        assert!(sut.is_trailing(&doc_grapheme(4, 0, " ")));
    }

    #[test]
    fn test_disabled_tracker() {
        let text = Rope::from_str("abc  ");
        let mut sut = TrailingWhitespaceTracker::new(false, text.slice(..));

        assert!(!sut.is_trailing(&doc_grapheme(4, 0, " ")));
    }
}
