use crate::{Category, Changelog, Entry, EntryKind, ErrorKind, Format, ParseError, Section};

use super::{
    document, histories, markdown, release,
    source::Source,
    text::{self, Entries},
};

pub fn parse(input: &str, name: Option<&str>) -> Result<Changelog, ParseError> {
    if name.is_some_and(fragment_name) {
        return fragment(input, name);
    }
    let source = Source::new(input);
    let lines: Vec<_> = source.lines().collect();
    let rst = lines.windows(2).any(|pair| match pair {
        [(_, title), (_, underline)] => adornment(underline) && is_release(title),
        _ => false,
    });
    if rst {
        return rst_history(input);
    }
    match histories::parse(input, Format::Towncrier) {
        Ok(history) => Ok(history),
        Err(error) if error.kind == ErrorKind::NoReleases => fragment(input, name),
        Err(error) => Err(error),
    }
}

fn fragment_name(name: &str) -> bool {
    let filename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let mut parts = filename.split('.');
    let (Some(_), Some(kind)) = (parts.next(), parts.next()) else {
        return false;
    };
    !matches!(kind.to_ascii_lowercase().as_str(), "md" | "rst" | "txt")
}

fn is_release(title: &str) -> bool {
    (histories::looks_like_release(title) || title.contains(" ("))
        && histories::label(title, Format::Towncrier).is_some()
}

fn adornment(text: &str) -> bool {
    text.len() >= 3
        && text.bytes().next().is_some_and(|first| {
            matches!(first, b'=' | b'-' | b'~' | b'^') && text.bytes().all(|byte| byte == first)
        })
}

fn rst_history(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let lines: Vec<_> = source.lines().collect();
    let mut changelog = document(input);
    let mut entries = Entries::default();
    let mut first = None;
    let mut index = 0usize;
    while let Some((span, line)) = lines.get(index).copied() {
        if let Some((underline_span, underline)) = index
            .checked_add(1)
            .and_then(|index| lines.get(index))
            .copied()
            && !line.trim().is_empty()
            && !line.starts_with(char::is_whitespace)
            && adornment(underline)
        {
            if let Some(current) = changelog.releases.last_mut() {
                entries.finish(&source, current, span.start)?;
            }
            let heading_span = source.span(span.start..underline_span.end);
            if is_release(line) {
                let label = histories::label(line, Format::Towncrier).ok_or_else(|| {
                    ParseError::new(
                        ErrorKind::MalformedHeading,
                        Some(span.line),
                        "invalid Towncrier release heading",
                    )
                })?;
                let mut current = release(
                    label.version,
                    source.text(text::range(heading_span))?.trim().to_owned(),
                    heading_span,
                );
                current.date = label.date;
                first.get_or_insert(span.start);
                changelog.releases.push(current);
            } else if let Some(current) = changelog.releases.last_mut() {
                current.sections.push(Section {
                    component: None,
                    category: Some(histories::category(line, Format::Towncrier)),
                    heading: Some(source.text(text::range(heading_span))?.trim().to_owned()),
                    entries: Vec::new(),
                    span: heading_span,
                });
            } else if changelog.metadata.title.is_none() {
                changelog.metadata.title = Some(line.to_owned());
            }
            index = index.saturating_add(2);
        } else {
            if let Some(current) = changelog.releases.last_mut() {
                entries.line(
                    &source,
                    current,
                    span,
                    line,
                    line.starts_with("- ") || line.starts_with("* "),
                )?;
            }
            index = index.saturating_add(1);
        }
    }
    if let Some(current) = changelog.releases.last_mut() {
        entries.finish(&source, current, input.len())?;
    }
    let first = first.ok_or_else(|| {
        ParseError::new(
            ErrorKind::NoReleases,
            None,
            "no Towncrier release headings found",
        )
    })?;
    source
        .text(0..first)?
        .trim()
        .clone_into(&mut changelog.metadata.description);
    Ok(changelog)
}

fn fragment(input: &str, name: Option<&str>) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let mut changelog = if name.is_some_and(|name| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    }) {
        markdown::body(input)?
    } else {
        let mut changelog = document(input);
        let mut current = release(None, String::new(), source.span(0..input.len()));
        if !input.trim().is_empty() {
            super::push_entry(
                &mut current,
                Entry {
                    markdown: input.trim().to_owned(),
                    kind: EntryKind::Text,
                    span: source.span(0..input.len()),
                },
            );
        }
        changelog.releases.push(current);
        changelog
    };
    if let Some(current) = changelog.releases.first_mut() {
        current.attributes.remove("version-status");
        current
            .attributes
            .insert("fragment".to_owned(), "towncrier".to_owned());
        if let Some(name) = name {
            let filename = name.rsplit(['/', '\\']).next().unwrap_or(name);
            let mut parts = filename.split('.');
            let (Some(issue), Some(kind)) = (parts.next(), parts.next()) else {
                return Err(ParseError::new(
                    ErrorKind::MalformedInput,
                    Some(1),
                    "Towncrier fragment name must be ISSUE.TYPE[.suffix]",
                ));
            };
            if issue.is_empty() || kind.is_empty() {
                return Err(ParseError::new(
                    ErrorKind::MalformedInput,
                    Some(1),
                    "Towncrier fragment issue and type must not be empty",
                ));
            }
            if !fragment_name(name) {
                if input.trim().is_empty() {
                    return Err(ParseError::new(
                        ErrorKind::EmptyInput,
                        Some(1),
                        "only misc fragments may have an empty body",
                    ));
                }
                return Ok(changelog);
            }
            current
                .attributes
                .insert("fragment-name".to_owned(), filename.to_owned());
            current
                .attributes
                .insert("type".to_owned(), kind.to_owned());
            if !issue.starts_with('+') {
                current
                    .attributes
                    .insert("issue".to_owned(), issue.to_owned());
            }
            let category = match kind {
                "feature" => Category::Added,
                "bugfix" => Category::Fixed,
                "removal" => Category::Removed,
                _ => Category::Custom(kind.to_owned()),
            };
            for section in &mut current.sections {
                if section.category.is_none() {
                    section.category = Some(category.clone());
                }
            }
            if input.trim().is_empty() && kind != "misc" {
                return Err(ParseError::new(
                    ErrorKind::EmptyInput,
                    Some(1),
                    "only misc fragments may have an empty body",
                ));
            }
        }
    }
    Ok(changelog)
}
