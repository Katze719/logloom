use std::ops::Range;

use crate::{Changelog, Entry, EntryKind, ParseError, Release, Span};

use super::{push_entry, source::Source};

#[derive(Default)]
pub struct Entries {
    pending: Option<(usize, EntryKind)>,
}

impl Entries {
    pub fn line(
        &mut self,
        source: &Source<'_>,
        current: &mut Release,
        span: Span,
        text: &str,
        item: bool,
    ) -> Result<(), ParseError> {
        if text.trim().is_empty() {
            if matches!(self.pending, Some((_, EntryKind::Text))) {
                self.finish(source, current, span.start)?;
            }
        } else {
            if item {
                self.finish(source, current, span.start)?;
            }
            self.pending.get_or_insert((
                span.start,
                if item {
                    EntryKind::ListItem
                } else {
                    EntryKind::Text
                },
            ));
        }
        Ok(())
    }

    pub fn finish(
        &mut self,
        source: &Source<'_>,
        current: &mut Release,
        end: usize,
    ) -> Result<(), ParseError> {
        if let Some((start, kind)) = self.pending.take() {
            push_entry(
                current,
                Entry {
                    markdown: source.text(start..end)?.trim().to_owned(),
                    kind,
                    span: source.span(start..end),
                },
            );
        }
        Ok(())
    }
}

pub fn rebase(
    changelog: &mut Changelog,
    original: &Source<'_>,
    offset: usize,
) -> Result<(), ParseError> {
    let shift = |span: &mut Span| -> Result<(), ParseError> {
        let start = offset.checked_add(span.start);
        let end = offset.checked_add(span.end);
        let range = start
            .zip(end)
            .map(|(start, end)| start..end)
            .ok_or_else(|| {
                crate::ParseError::new(
                    crate::ErrorKind::ResourceLimit,
                    None,
                    "source offset overflow",
                )
            })?;
        original.text(range.clone())?;
        *span = original.span(range);
        Ok(())
    };
    for release in &mut changelog.releases {
        shift(&mut release.span)?;
        if let Some(span) = &mut release.date_span {
            shift(span)?;
        }
        for link in &mut release.links {
            shift(&mut link.span)?;
        }
        for section in &mut release.sections {
            shift(&mut section.span)?;
            for entry in &mut section.entries {
                shift(&mut entry.span)?;
            }
        }
    }
    for link in &mut changelog.links {
        shift(&mut link.span)?;
    }
    original.input.clone_into(&mut changelog.source);
    Ok(())
}

pub fn containing_spans(changelog: &mut Changelog, span: Span) {
    for release in &mut changelog.releases {
        release.span = span;
        if let Some(date_span) = &mut release.date_span {
            *date_span = span;
        }
        for link in &mut release.links {
            link.span = span;
        }
        for section in &mut release.sections {
            section.span = span;
            for entry in &mut section.entries {
                entry.span = span;
            }
        }
    }
    for link in &mut changelog.links {
        link.span = span;
    }
}

pub const fn range(span: Span) -> Range<usize> {
    span.start..span.end
}
