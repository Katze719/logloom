use crate::{Category, Changelog, ErrorKind, ParseError, Release, Section, Span};

use super::{document, release, source::Source, text::Entries};

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let mut changelog = document(input);
    let mut current = None;
    let mut entries = Entries::default();
    for (span, line) in source.lines() {
        if line.is_empty() {
            if let Some(current) = &mut current {
                entries.line(&source, current, span, line, false)?;
            }
            continue;
        }
        if let Some(trailer) = line.strip_prefix(" -- ") {
            let Some(mut finished) = current.take() else {
                return Err(malformed(
                    span.line,
                    "maintainer trailer appears before a Debian release header",
                ));
            };
            entries.finish(&source, &mut finished, span.start)?;
            let (maintainer, timestamp) = trailer
                .rsplit_once("  ")
                .filter(|(name, date)| {
                    !name.is_empty() && !name.ends_with(' ') && !date.starts_with(' ')
                })
                .ok_or_else(|| {
                    malformed(
                        span.line,
                        "separate maintainer and RFC 2822 date with exactly two spaces",
                    )
                })?;
            finished.date = Some(
                debian_date(timestamp)
                    .ok_or_else(|| malformed(span.line, "invalid Debian RFC 2822 timestamp"))?,
            );
            finished.date_span = Some(span);
            finished
                .attributes
                .insert("maintainer".to_owned(), maintainer.to_owned());
            finished
                .attributes
                .insert("timestamp".to_owned(), timestamp.to_owned());
            changelog.releases.push(finished);
        } else if line.starts_with("  ") {
            let Some(current) = &mut current else {
                return Err(malformed(
                    span.line,
                    "change details appear outside a Debian release record",
                ));
            };
            if let Some(author) = line
                .strip_prefix("  [")
                .and_then(|line| line.strip_suffix(']'))
            {
                entries.finish(&source, current, span.start)?;
                current.sections.push(Section {
                    component: None,
                    category: Some(Category::Custom(author.trim().to_owned())),
                    heading: Some(line.to_owned()),
                    entries: Vec::new(),
                    span,
                });
            } else {
                entries.line(&source, current, span, line, line.starts_with("  * "))?;
            }
        } else {
            if current.is_some() {
                return Err(malformed(
                    span.line,
                    "previous Debian release is missing its maintainer trailer",
                ));
            }
            let next = header(line, span)?;
            if changelog.metadata.title.is_none() {
                changelog.metadata.title = next.attributes.get("package").cloned();
            }
            current = Some(next);
        }
    }
    if current.is_some() {
        return Err(ParseError::new(
            ErrorKind::MalformedInput,
            current.as_ref().map(|release| release.span.line),
            "Debian release is missing its maintainer trailer",
        ));
    }
    if changelog.releases.is_empty() {
        return Err(ParseError::new(
            ErrorKind::NoReleases,
            None,
            "no Debian release records found",
        ));
    }
    Ok(changelog)
}

fn malformed(line: usize, message: &str) -> ParseError {
    ParseError::new(ErrorKind::MalformedInput, Some(line), message)
}

fn debian_date(timestamp: &str) -> Option<String> {
    let fields: Vec<_> = timestamp.split_whitespace().collect();
    let [weekday, day, month, year, time, zone] = fields.as_slice() else {
        return None;
    };
    if !matches!(
        *weekday,
        "Mon," | "Tue," | "Wed," | "Thu," | "Fri," | "Sat," | "Sun,"
    ) || year.len() != 4
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || !(1..=2).contains(&day.len())
        || !day.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let month = match *month {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day = day.parse::<u8>().ok()?;
    let year = year.parse::<u16>().ok()?;
    if !time
        .split(':')
        .all(|part| part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    let clock: Vec<_> = time
        .split(':')
        .map(str::parse::<u8>)
        .collect::<Result<_, _>>()
        .ok()?;
    if !matches!(clock.as_slice(), [hour, minute, second] if *hour<24 && *minute<60 && *second<=60)
    {
        return None;
    }
    let zone_digits = zone.strip_prefix(['+', '-'])?;
    if zone_digits.len() != 4 || !zone_digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let zone_hour = zone_digits.get(0..2)?.parse::<u8>().ok()?;
    let zone_minute = zone_digits.get(2..4)?.parse::<u8>().ok()?;
    if zone_hour > 23 || zone_minute > 59 {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn header(line: &str, span: Span) -> Result<Release, ParseError> {
    let (package, rest) = line.split_once(" (").ok_or_else(|| {
        malformed(
            span.line,
            "expected package (version) distributions; urgency=level",
        )
    })?;
    let (version, rest) = rest
        .split_once(") ")
        .ok_or_else(|| malformed(span.line, "Debian version must be enclosed in parentheses"))?;
    let (distributions, settings) = rest.split_once(';').ok_or_else(|| {
        malformed(
            span.line,
            "Debian header must include distributions and urgency",
        )
    })?;
    if package.is_empty()
        || version.is_empty()
        || distributions.trim().is_empty()
        || package.contains(char::is_whitespace)
    {
        return Err(malformed(
            span.line,
            "package, version, and distributions must not be empty",
        ));
    }
    let mut next = release(Some(version.to_owned()), line.to_owned(), span);
    next.attributes
        .insert("package".to_owned(), package.to_owned());
    next.attributes
        .insert("distributions".to_owned(), distributions.trim().to_owned());
    for setting in settings.split(',') {
        let (key, value) = setting
            .trim()
            .split_once('=')
            .filter(|(key, value)| !key.is_empty() && !value.is_empty())
            .ok_or_else(|| {
                malformed(
                    span.line,
                    "Debian settings must use nonempty key=value pairs",
                )
            })?;
        if next
            .attributes
            .insert(key.to_owned(), value.to_owned())
            .is_some()
        {
            return Err(malformed(span.line, "duplicate Debian header setting"));
        }
    }
    if !next.attributes.contains_key("urgency") {
        return Err(malformed(span.line, "Debian header is missing urgency"));
    }
    Ok(next)
}
