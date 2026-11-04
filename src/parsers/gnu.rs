use crate::{Category, Changelog, ErrorKind, Format, ParseError, Section};

use super::{document, histories, release, source::Source, text::Entries};

pub fn parse(input: &str, format: Format) -> Result<Changelog, ParseError> {
    if format == Format::GnuChangeLog {
        change_log(input)
    } else {
        news(input)
    }
}

fn news(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let mut changelog = document(input);
    let mut entries = Entries::default();
    let mut first = None;
    for (span, line) in source.lines() {
        let title = line.trim_start_matches('*').trim();
        let version = [
            "Noteworthy changes in release ",
            "Changes in version ",
            "Changes in release ",
            "Version ",
        ]
        .iter()
        .find_map(|prefix| title.strip_prefix(prefix));
        if let Some(version) = version {
            if let Some(current) = changelog.releases.last_mut() {
                entries.finish(&source, current, span.start)?;
            }
            let label = news_label(version).ok_or_else(|| {
                ParseError::new(
                    ErrorKind::MalformedHeading,
                    Some(span.line),
                    "invalid GNU NEWS release heading",
                )
            })?;
            let mut current = release(label.version, line.to_owned(), span);
            current.date = label.date;
            first.get_or_insert(span.start);
            changelog.releases.push(current);
        } else if line.starts_with("** ") {
            if let Some(current) = changelog.releases.last_mut() {
                entries.finish(&source, current, span.start)?;
                let category = match title.to_ascii_lowercase().as_str() {
                    "bug fixes" => Category::Fixed,
                    "new features" => Category::Added,
                    _ => Category::from_name(title),
                };
                current.sections.push(Section {
                    component: None,
                    category: Some(category),
                    heading: Some(line.to_owned()),
                    entries: Vec::new(),
                    span,
                });
            }
        } else if let Some(current) = changelog.releases.last_mut() {
            entries.line(
                &source,
                current,
                span,
                line,
                line.trim_start().starts_with("- ") || line.trim_start().starts_with("* "),
            )?;
        }
    }
    if let Some(current) = changelog.releases.last_mut() {
        entries.finish(&source, current, input.len())?;
    }
    let first = first.ok_or_else(|| {
        ParseError::new(
            ErrorKind::NoReleases,
            None,
            "no GNU NEWS release headings found",
        )
    })?;
    source
        .text(0..first)?
        .trim()
        .clone_into(&mut changelog.metadata.description);
    changelog.metadata.title = input
        .lines()
        .next()
        .filter(|line| line.starts_with("GNU "))
        .map(str::to_owned);
    Ok(changelog)
}

fn news_label(text: &str) -> Option<histories::Label> {
    let text = if let Some((name, rest)) = text.split_once(" (") {
        let (date, _) = rest.split_once(')')?;
        format!("{name} ({date})")
    } else {
        text.to_owned()
    };
    let mut label = histories::label(&text, Format::GenericMarkdown)?;
    if label
        .version
        .as_ref()
        .is_some_and(|version| version.contains('?'))
    {
        label.version = None;
    }
    if label.date.as_ref().is_some_and(|date| date.contains('?')) {
        label.date = None;
    }
    Some(label)
}

fn change_log(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let mut changelog = document(input);
    let mut entries = Entries::default();
    let mut first = None;
    for (span, line) in source.lines() {
        let header = line
            .get(0..10)
            .zip(line.get(10..))
            .filter(|(date, rest)| date_shape(date) && rest.starts_with(char::is_whitespace));
        if let Some((date, author)) = header {
            if author.trim().is_empty() {
                return Err(ParseError::new(
                    ErrorKind::MalformedHeading,
                    Some(span.line),
                    "GNU ChangeLog date header is missing an author",
                ));
            }
            if let Some(current) = changelog.releases.last_mut() {
                entries.finish(&source, current, span.start)?;
            }
            let mut current = release(None, line.to_owned(), span);
            current.date = Some(date.to_owned());
            current
                .attributes
                .insert("record-kind".to_owned(), "dated-change".to_owned());
            current
                .attributes
                .insert("author".to_owned(), author.trim().to_owned());
            first.get_or_insert(span.start);
            changelog.releases.push(current);
        } else if let Some(current) = changelog.releases.last_mut() {
            entries.line(
                &source,
                current,
                span,
                line,
                line.trim_start().starts_with("* "),
            )?;
        }
    }
    if let Some(current) = changelog.releases.last_mut() {
        entries.finish(&source, current, input.len())?;
    }
    let first = first.ok_or_else(|| {
        ParseError::new(
            ErrorKind::NoReleases,
            None,
            "no ISO date-and-author GNU ChangeLog records found",
        )
    })?;
    source
        .text(0..first)?
        .trim()
        .clone_into(&mut changelog.metadata.description);
    Ok(changelog)
}

fn date_shape(text: &str) -> bool {
    let mut parts = text.split('-');
    matches!((parts.next(), parts.next(), parts.next(), parts.next()), (Some(year), Some(month), Some(day), None) if year.len()==4 && month.len()==2 && day.len()==2 && [year,month,day].iter().all(|part| part.bytes().all(|byte| byte.is_ascii_digit())))
}
