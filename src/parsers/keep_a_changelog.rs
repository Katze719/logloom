use std::{collections::HashSet, ops::Range};

use pulldown_cmark::HeadingLevel;

use super::markdown::{self, Block, Heading};

use crate::{
    Category, Changelog, Entry, EntryKind, ErrorKind, LinkDefinition, MAX_INPUT_BYTES, Metadata,
    ParseError, Release, Section, Span,
};

struct Document<'a> {
    input: &'a str,
    lines: Vec<usize>,
    metadata: Metadata,
    releases: Vec<Release>,
    title_range: Option<Range<usize>>,
    first_release: Option<usize>,
    component: Option<String>,
    needs_component_section: bool,
    component_name_bytes: usize,
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
            component: None,
            needs_component_section: false,
            component_name_bytes: 0,
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
                self.component = None;
                self.needs_component_section = false;
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
                    date_span: None,
                    yanked,
                    heading: self.source(heading.range)?.trim().to_owned(),
                    link: heading.link,
                    sections: Vec::new(),
                    span,
                    attributes: std::collections::BTreeMap::new(),
                    links: Vec::new(),
                });
            }
            HeadingLevel::H3 => {
                self.component = None;
                self.category_heading(&heading)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn component_heading(&mut self, heading: &Heading) -> Result<(), ParseError> {
        if self.releases.is_empty() {
            return Err(self.error(
                ErrorKind::UnsupportedStructure,
                &heading.range,
                "component heading appears before any release",
            ));
        }
        let name = heading.text.trim();
        if name.is_empty() {
            return Err(self.error(
                ErrorKind::MalformedHeading,
                &heading.range,
                "component name is empty",
            ));
        }
        self.component = Some(name.to_owned());
        self.needs_component_section = true;
        Ok(())
    }

    fn category_heading(&mut self, heading: &Heading) -> Result<(), ParseError> {
        let span = self.span(&heading.range);
        if heading.text.trim().is_empty() {
            return Err(self.error(
                ErrorKind::MalformedHeading,
                &heading.range,
                "category name is empty",
            ));
        }
        let raw = self.source(heading.range.clone())?.trim().to_owned();
        let component = self.section_component(span.line)?;
        let release = self.releases.last_mut().ok_or_else(|| {
            ParseError::new(
                ErrorKind::UnsupportedStructure,
                Some(span.line),
                "category heading appears before any release",
            )
        })?;
        release.sections.push(Section {
            component,
            category: Some(Category::from_name(heading.text.trim())),
            heading: Some(raw),
            entries: Vec::new(),
            span,
        });
        self.needs_component_section = false;
        Ok(())
    }

    fn section_component(&mut self, line: usize) -> Result<Option<String>, ParseError> {
        let Some(name) = &self.component else {
            return Ok(None);
        };
        let bytes = self
            .component_name_bytes
            .checked_add(name.len())
            .filter(|bytes| *bytes <= MAX_INPUT_BYTES)
            .ok_or_else(|| {
                ParseError::new(
                    ErrorKind::ResourceLimit,
                    Some(line),
                    "expanded component names exceed the 16 MiB limit",
                )
            })?;
        self.component_name_bytes = bytes;
        Ok(Some(name.clone()))
    }

    fn entry(&mut self, range: Range<usize>, kind: EntryKind) -> Result<(), ParseError> {
        if self.releases.is_empty() {
            return Ok(());
        }
        let span = self.span(&range);
        let markdown = self.source(range)?.trim().to_owned();
        let needs_section = self.needs_component_section
            || self
                .releases
                .last()
                .is_some_and(|release| release.sections.is_empty());
        let component = if needs_section {
            self.section_component(span.line)?
        } else {
            None
        };
        if let Some(release) = self.releases.last_mut() {
            if needs_section {
                release.sections.push(Section {
                    component,
                    category: None,
                    heading: None,
                    entries: Vec::new(),
                    span,
                });
                self.needs_component_section = false;
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
            format: crate::Format::KeepAChangelog,
            metadata: self.metadata,
            releases: self.releases,
            links,
            source: self.input.to_owned(),
        })
    }
}

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    let mut document = Document::new(input);
    let markdown = markdown::scan(input)?;
    let components = component_headings(&markdown.blocks);
    for block in markdown.blocks {
        match block {
            Block::Heading(heading) if components.contains(&heading.range.start) => {
                document.component_heading(&heading)?;
            }
            Block::Heading(heading)
                if heading.level == HeadingLevel::H4 && document.component.is_some() =>
            {
                document.category_heading(&heading)?;
            }
            Block::Heading(heading)
                if matches!(
                    heading.level,
                    HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                ) =>
            {
                document.heading(heading)?;
            }
            Block::Heading(heading) => document.entry(heading.range, EntryKind::Markdown)?,
            Block::Entry(range, kind) => document.entry(range, kind)?,
        }
    }
    document.finish(markdown.links)
}

fn component_headings(blocks: &[Block]) -> HashSet<usize> {
    let mut component = None;
    let mut components = HashSet::new();
    for block in blocks {
        if let Block::Heading(heading) = block {
            match heading.level {
                HeadingLevel::H1 | HeadingLevel::H2 => component = None,
                HeadingLevel::H3 => {
                    component = if matches!(
                        Category::from_name(heading.text.trim()),
                        Category::Custom(_)
                    ) {
                        Some(heading.range.start)
                    } else {
                        None
                    };
                }
                HeadingLevel::H4 => {
                    if let Some(start) = component {
                        components.insert(start);
                    }
                }
                _ => {}
            }
        }
    }
    components
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
