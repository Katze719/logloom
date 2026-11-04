use pulldown_cmark::HeadingLevel;

use crate::{Category, Changelog, Entry, ErrorKind, Format, ParseError, Section};

use super::{
    document,
    markdown::{self, Block},
    push_entry, release,
    source::Source,
};

pub struct Label {
    pub version: Option<String>,
    pub date: Option<String>,
    pub yanked: bool,
}

pub fn looks_like_release(text: &str) -> bool {
    let text = text.trim().trim_start_matches('[');
    let text = text.strip_prefix('v').unwrap_or(text);
    text.starts_with(|character: char| character.is_ascii_digit())
        || text.eq_ignore_ascii_case("unreleased")
        || text.eq_ignore_ascii_case("next")
}

pub fn label(text: &str, format: Format) -> Option<Label> {
    let text = text.trim();
    let (text, yanked) = text
        .strip_suffix("[YANKED]")
        .map_or((text, false), |rest| (rest.trim_end(), true));
    let (name, date) = if let Some((name, date)) = text.rsplit_once(" (") {
        (name.trim(), Some(date.strip_suffix(')')?.trim()))
    } else if let Some((name, date)) = text.rsplit_once(" - ") {
        (name.trim(), Some(date.trim()))
    } else {
        (text, None)
    };
    if date.is_some_and(str::is_empty) {
        return None;
    }
    let name = name
        .strip_prefix('[')
        .map_or(Some(name), |name| name.strip_suffix(']'))?;
    let name = if format == Format::Towncrier {
        name.split_whitespace().last()?
    } else {
        name.strip_prefix("Version ")
            .or_else(|| name.strip_prefix("Release "))
            .unwrap_or(name)
    };
    if name.is_empty()
        || (format != Format::GenericMarkdown
            && (name.contains(char::is_whitespace) || name.contains(['[', ']'])))
    {
        return None;
    }
    let pending = name.eq_ignore_ascii_case("unreleased") || name.eq_ignore_ascii_case("next");
    Some(Label {
        version: if pending { None } else { Some(name.to_owned()) },
        date: date.map(str::to_owned),
        yanked,
    })
}

pub fn parse(input: &str, format: Format) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let markdown = markdown::scan(input)?;
    let first = markdown.blocks.iter().find_map(|block| match block {
        Block::Heading(heading) if is_release(heading.level, &heading.text, format) => {
            Some(heading.range.start)
        }
        _ => None,
    });
    let Some(first) = first else {
        if format == Format::GenericMarkdown {
            return unversioned(input);
        }
        return Err(ParseError::new(
            ErrorKind::NoReleases,
            None,
            "no release headings recognized for the selected format",
        ));
    };
    let mut changelog = document(input);
    changelog.links = markdown.links;
    let mut title_range = None;
    for block in markdown.blocks {
        match block {
            Block::Heading(heading) if is_release(heading.level, &heading.text, format) => {
                let span = source.span(heading.range.clone());
                let label = label(&heading.text, format).ok_or_else(|| {
                    ParseError::new(
                        ErrorKind::MalformedHeading,
                        Some(span.line),
                        "release heading has an invalid version or date delimiter",
                    )
                })?;
                let mut current = release(
                    label.version,
                    source.text(heading.range)?.trim().to_owned(),
                    span,
                );
                current.date = label.date;
                current.yanked = label.yanked;
                current.link = heading.link;
                if format == Format::Changesets
                    && let Some(package) = &changelog.metadata.title
                {
                    current
                        .attributes
                        .insert("package".to_owned(), package.clone());
                }
                changelog.releases.push(current);
            }
            Block::Heading(heading) => {
                if let Some(current) = changelog.releases.last_mut() {
                    current.sections.push(Section {
                        component: None,
                        category: Some(category(&heading.text, format)),
                        heading: Some(source.text(heading.range.clone())?.trim().to_owned()),
                        entries: Vec::new(),
                        span: source.span(heading.range),
                    });
                } else if heading.level == HeadingLevel::H1 && changelog.metadata.title.is_none() {
                    changelog.metadata.title = Some(heading.text);
                    title_range = Some(heading.range);
                }
            }
            Block::Entry(range, kind) => {
                if let Some(current) = changelog.releases.last_mut() {
                    push_entry(
                        current,
                        Entry {
                            markdown: source.text(range.clone())?.trim().to_owned(),
                            kind,
                            span: source.span(range),
                        },
                    );
                }
            }
        }
    }
    changelog.metadata.description = if let Some(title) = title_range {
        format!(
            "{}{}",
            source.text(0..title.start)?,
            source.text(title.end..first)?
        )
        .trim()
        .to_owned()
    } else {
        source.text(0..first)?.trim().to_owned()
    };
    Ok(changelog)
}

fn unversioned(input: &str) -> Result<Changelog, ParseError> {
    let mut changelog = markdown::body(input)?;
    if let Some(current) = changelog.releases.first_mut()
        && let Some(section) = current.sections.first_mut()
        && section
            .heading
            .as_ref()
            .is_some_and(|heading| heading.starts_with("# "))
    {
        changelog.metadata.title = section.category.as_ref().map(|category| match category {
            Category::Custom(name) => name.clone(),
            _ => section
                .heading
                .as_deref()
                .unwrap_or_default()
                .trim_start_matches('#')
                .trim()
                .to_owned(),
        });
        section.category = None;
        section.heading = None;
    }
    Ok(changelog)
}

pub fn category(name: &str, format: Format) -> Category {
    match (format, name.trim().to_ascii_lowercase().as_str()) {
        (Format::ConventionalChangelog | Format::Towncrier, "features") => Category::Added,
        (Format::ConventionalChangelog, "bug fixes") | (Format::Towncrier, "bugfixes") => {
            Category::Fixed
        }
        (Format::ConventionalChangelog, "performance improvements") => Category::Changed,
        _ => Category::from_name(name.trim()),
    }
}

fn is_release(level: HeadingLevel, text: &str, format: Format) -> bool {
    let shallow = match format {
        Format::GenericMarkdown => matches!(level, HeadingLevel::H1 | HeadingLevel::H2),
        Format::Changesets => level == HeadingLevel::H2,
        _ => matches!(
            level,
            HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
        ),
    };
    shallow
        && (looks_like_release(text)
            || (format == Format::GenericMarkdown && level == HeadingLevel::H2)
            || (format == Format::Towncrier
                && text.contains(" (")
                && label(text, format).is_some()))
}
