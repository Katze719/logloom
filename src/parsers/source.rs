use std::ops::Range;

use crate::{ErrorKind, ParseError, Span};

pub struct Source<'a> {
    pub input: &'a str,
    offsets: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut offsets = vec![0];
        offsets.extend(
            input
                .match_indices('\n')
                .filter_map(|(offset, _)| offset.checked_add(1)),
        );
        Self { input, offsets }
    }

    pub fn span(&self, range: Range<usize>) -> Span {
        Span {
            start: range.start,
            end: range.end,
            line: self
                .offsets
                .partition_point(|offset| *offset <= range.start),
        }
    }

    pub fn text(&self, range: Range<usize>) -> Result<&'a str, ParseError> {
        self.input
            .get(range)
            .ok_or_else(|| ParseError::new(ErrorKind::MalformedInput, None, "invalid source range"))
    }

    pub fn lines(&self) -> impl Iterator<Item = (Span, &'a str)> + '_ {
        self.input
            .split_inclusive('\n')
            .scan(0usize, |start, text| {
                let end = start.checked_add(text.len())?;
                let span = self.span(*start..end);
                *start = end;
                Some((span, text.trim_end_matches(['\r', '\n'])))
            })
    }
}
