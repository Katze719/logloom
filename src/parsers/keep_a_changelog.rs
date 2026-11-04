use std::ops::Range;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    Category, Changelog, Entry, EntryKind, ErrorKind, LinkDefinition, Metadata, ParseError,
    Release, Section, Span,
};

struct Heading {
    level: HeadingLevel,
    text: String,
    link: Option<String>,
    range: Range<usize>,
}

struct Document<'a> {
    input: &'a str,
    lines: Vec<usize>,
    metadata: Metadata,
    releases: Vec<Release>,
    title_range: Option<Range<usize>>,
    first_release: Option<usize>,
}

impl<'a> Document<'a> {
    fn new(input: &'a str) -> Self {
        let mut lines = vec![0];
        lines.extend(
            input
                .match_indices('\n')
                .filter_map(|(offset, _)| offset.checked_add(1)),
        );
        Self {
            input,
            lines,
            metadata: Metadata {
                title: None,
                description: String::new(),
            },
            releases: Vec::new(),
            title_range: None,
            first_release: None,
        }
    }

    fn span(&self, range: &Range<usize>) -> Span {
        Span {
            start: range.start,
            end: range.end,
            line: self.lines.partition_point(|offset| *offset <= range.start),
        }
    }

    fn error(&self, kind: ErrorKind, range: &Range<usize>, message: &str) -> ParseError {
        ParseError::new(kind, Some(self.span(range).line), message)
    }

    fn source(&self, range: Range<usize>) -> Result<&'a str, ParseError> {
        self.input.get(range.clone()).ok_or_else(|| {
            self.error(
                ErrorKind::UnsupportedStructure,
                &range,
                "Markdown source range is invalid",
            )
        })
    }

    fn heading(&mut self, heading: Heading) -> Result<(), ParseError> {
        let span = self.span(&heading.range);
        match heading.level {
            HeadingLevel::H1 => {
                if self.metadata.title.is_some() || !self.releases.is_empty() {
                    return Err(self.error(
                        ErrorKind::UnsupportedStructure,
                        &heading.range,
                        "only one document title is supported, before all releases",
                    ));
                }
                self.metadata.title = Some(heading.text.trim().to_owned());
                self.title_range = Some(heading.range);
            }
            HeadingLevel::H2 => {
                let (version, date, yanked) = release_label(&heading.text).ok_or_else(|| {
                    self.error(
                        ErrorKind::MalformedHeading,
                        &heading.range,
                        "expected '## [version] - YYYY-MM-DD', '## version', or '## [Unreleased]'",
                    )
                })?;
                self.first_release.get_or_insert(heading.range.start);
                self.releases.push(Release {
                    version,
                    date,
                    yanked,
                    heading: self.source(heading.range)?.trim().to_owned(),
                    link: heading.link,
                    sections: Vec::new(),
                    span,
                });
            }
            HeadingLevel::H3 => {
                let raw = self.source(heading.range.clone())?.trim().to_owned();
                let release = self.releases.last_mut().ok_or_else(|| {
                    ParseError::new(
                        ErrorKind::UnsupportedStructure,
                        Some(span.line),
                        "category heading appears before any release",
                    )
                })?;
                if heading.text.trim().is_empty() {
                    return Err(ParseError::new(
                        ErrorKind::MalformedHeading,
                        Some(span.line),
                        "category name is empty",
                    ));
                }
                release.sections.push(Section {
                    category: Some(Category::from_name(heading.text.trim())),
                    heading: Some(raw),
                    entries: Vec::new(),
                    span,
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn entry(&mut self, range: Range<usize>, kind: EntryKind) -> Result<(), ParseError> {
        if self.releases.is_empty() {
            return Ok(());
        }
        let span = self.span(&range);
        let markdown = self.source(range)?.trim().to_owned();
        if let Some(release) = self.releases.last_mut() {
            if release.sections.is_empty() {
                release.sections.push(Section {
                    category: None,
                    heading: None,
                    entries: Vec::new(),
                    span,
                });
            }
            if let Some(section) = release.sections.last_mut() {
                section.entries.push(Entry {
                    markdown,
                    kind,
                    span,
                });
            }
        }
        Ok(())
    }

    fn finish(mut self, links: Vec<LinkDefinition>) -> Result<Changelog, ParseError> {
        let end = self.first_release.ok_or_else(|| {
            ParseError::new(
                ErrorKind::NoReleases,
                None,
                "no releases found; use a level-two heading such as '## [Unreleased]'",
            )
        })?;
        self.metadata.description = if let Some(title) = self.title_range.clone() {
            // Preserve text both before and after a title, even when it is unconventional.
            let before = self.source(0..title.start)?;
            let after = self.source(title.end..end)?;
            format!("{before}{after}").trim().to_owned()
        } else {
            self.source(0..end)?.trim().to_owned()
        };
        Ok(Changelog {
            metadata: self.metadata,
            releases: self.releases,
            links,
            source: self.input.to_owned(),
        })
    }
}

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    let mut document = Document::new(input);
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(input, options);
    let mut links: Vec<_> = parser
        .reference_definitions()
        .iter()
        .map(|(label, definition)| LinkDefinition {
            label: label.to_owned(),
            destination: definition.dest.to_string(),
            title: definition.title.as_ref().map(ToString::to_string),
            span: document.span(&definition.span),
        })
        .collect();
    links.sort_by_key(|link| link.span.start);
    let mut stack = Vec::new();
    let mut heading: Option<Heading> = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if stack.len() >= 128 {
                    return Err(document.error(
                        ErrorKind::ResourceLimit,
                        &range,
                        "Markdown nesting exceeds 128 containers",
                    ));
                }
                start_block(&mut document, &tag, &stack, &range, &mut heading)?;
                if let (Some(heading), Tag::Link { dest_url, .. }) = (&mut heading, &tag) {
                    heading.link.get_or_insert_with(|| dest_url.to_string());
                }
                stack.push(tag.to_end());
            }
            Event::End(tag) => {
                stack.pop();
                if stack.is_empty()
                    && matches!(tag, TagEnd::Heading(_))
                    && let Some(heading) = heading.take()
                {
                    document.heading(heading)?;
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = &mut heading {
                    heading.text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = &mut heading {
                    heading.text.push(' ');
                }
            }
            Event::Rule if stack.is_empty() => document.entry(range, EntryKind::Markdown)?,
            _ => {}
        }
    }
    document.finish(links)
}

fn start_block(
    document: &mut Document<'_>,
    tag: &Tag<'_>,
    stack: &[TagEnd],
    range: &Range<usize>,
    heading: &mut Option<Heading>,
) -> Result<(), ParseError> {
    if stack.is_empty() {
        match tag {
            Tag::Heading {
                level: level @ (HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3),
                ..
            } => {
                *heading = Some(Heading {
                    level: *level,
                    text: String::new(),
                    link: None,
                    range: range.clone(),
                });
            }
            Tag::List(_) => {}
            _ => document.entry(range.clone(), EntryKind::Markdown)?,
        }
    } else if matches!(tag, Tag::Item) && matches!(stack, [TagEnd::List(_)]) {
        document.entry(range.clone(), EntryKind::ListItem)?;
    }
    Ok(())
}

type ReleaseLabel = (Option<String>, Option<String>, bool);

fn release_label(text: &str) -> Option<ReleaseLabel> {
    let text = text.trim();
    let (text, yanked) = text
        .strip_suffix("[YANKED]")
        .map_or((text, false), |rest| (rest.trim_end(), true));
    let (version, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let version = if version.starts_with('[') || version.ends_with(']') {
        version.strip_prefix('[')?.strip_suffix(']')?
    } else {
        version
    };
    if version.is_empty() || version.contains(['[', ']']) {
        return None;
    }
    let rest = rest.trim();
    let date = if rest.is_empty() {
        None
    } else {
        let date = rest.strip_prefix('-')?.trim();
        if date.is_empty() {
            return None;
        }
        Some(date.to_owned())
    };
    let version = if version.eq_ignore_ascii_case("unreleased") {
        None
    } else {
        Some(version.to_owned())
    };
    Some((version, date, yanked))
}
