use serde::Deserialize;
use serde_json::value::RawValue;

use crate::{Changelog, ErrorKind, Format, ParseError};

use super::{
    document, histories,
    markdown::{self, Block},
    source::Source,
    text,
};

#[derive(Deserialize)]
struct GitHubRelease<'a> {
    #[serde(alias = "tagName")]
    tag_name: String,
    name: Option<String>,
    #[serde(borrow)]
    body: Option<&'a RawValue>,
    #[serde(alias = "publishedAt")]
    #[serde(borrow)]
    published_at: Option<&'a RawValue>,
    #[serde(alias = "url")]
    html_url: Option<String>,
    #[serde(default, alias = "isDraft")]
    draft: bool,
    #[serde(default, alias = "isPrerelease")]
    prerelease: bool,
}

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    let trimmed = input.trim_start();
    let json_array = trimmed.strip_prefix('[').is_some_and(|rest| {
        rest.trim().is_empty()
            || rest.trim_start().starts_with(['{', '[', ']'])
            || serde_json::from_str::<&RawValue>(input).is_ok()
    });
    if trimmed.starts_with('{') || json_array {
        json(input)
    } else {
        let markdown = markdown::scan(input)?;
        if markdown.blocks.iter().any(|block| matches!(block, Block::Heading(heading) if histories::looks_like_release(&heading.text))) {
            histories::parse(input, Format::ConventionalChangelog)
        } else { markdown::body(input) }
    }
}

fn json(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let items: Vec<&RawValue> = if input.trim_start().starts_with('[') {
        serde_json::from_str(input).map_err(|error| json_error(&error))?
    } else {
        vec![serde_json::from_str(input).map_err(|error| json_error(&error))?]
    };
    if items.is_empty() {
        return Err(ParseError::new(
            ErrorKind::NoReleases,
            Some(1),
            "GitHub release array is empty",
        ));
    }
    let mut changelog = document(input);
    for raw in items {
        let parsed = json_item(&source, raw)?;
        changelog.links.extend(parsed.links);
        changelog.releases.extend(parsed.releases);
    }
    Ok(changelog)
}

fn json_item(source: &Source<'_>, raw: &RawValue) -> Result<Changelog, ParseError> {
    let item_span = raw_span(source, raw)?;
    let item: GitHubRelease<'_> = serde_json::from_str(raw.get()).map_err(|error| {
        let mut error = json_error(&error);
        error.line = Some(
            item_span
                .line
                .saturating_add(error.line.unwrap_or(1).saturating_sub(1)),
        );
        error
    })?;
    if item.tag_name.trim().is_empty() {
        return Err(ParseError::new(
            ErrorKind::MalformedInput,
            Some(item_span.line),
            "GitHub release tag_name must not be empty",
        ));
    }
    let body_span = raw_span(source, item.body.unwrap_or(raw))?;
    let body: Option<String> = item
        .body
        .map(|body| serde_json::from_str(body.get()))
        .transpose()
        .map_err(|error| {
            ParseError::new(
                ErrorKind::MalformedInput,
                Some(body_span.line),
                format!("GitHub body must be a string or null: {error}"),
            )
        })?
        .flatten();
    let mut parsed = markdown::body(body.as_deref().unwrap_or_default()).map_err(|mut error| {
        error.line = Some(body_span.line);
        error.message = format!("GitHub release body: {}", error.message);
        error
    })?;
    // Decoded Markdown offsets cannot index its escaped JSON representation.
    text::containing_spans(&mut parsed, body_span);
    let current = parsed.releases.first_mut().ok_or_else(|| {
        ParseError::new(
            ErrorKind::MalformedInput,
            None,
            "missing release body container",
        )
    })?;
    current.span = item_span;
    current.attributes.remove("version-status");
    current.heading = item.name.unwrap_or_else(|| item.tag_name.clone());
    current.version = Some(item.tag_name);
    current.link = item.html_url;
    if let Some(raw_timestamp) = item.published_at {
        let span = raw_span(source, raw_timestamp)?;
        let timestamp: String = serde_json::from_str(raw_timestamp.get()).map_err(|error| {
            ParseError::new(
                ErrorKind::MalformedInput,
                Some(span.line),
                format!("published_at must be a UTC ISO timestamp string: {error}"),
            )
        })?;
        let date = timestamp_date(&timestamp).ok_or_else(|| {
            ParseError::new(
                ErrorKind::MalformedInput,
                Some(span.line),
                "published_at must be a UTC ISO timestamp with a valid clock time",
            )
        })?;
        current.date = Some(date);
        current.date_span = Some(span);
        current
            .attributes
            .insert("published-at".to_owned(), timestamp);
    }
    current
        .attributes
        .insert("draft".to_owned(), item.draft.to_string());
    current
        .attributes
        .insert("prerelease".to_owned(), item.prerelease.to_string());
    Ok(parsed)
}

fn raw_span(source: &Source<'_>, raw: &RawValue) -> Result<crate::Span, ParseError> {
    let start = raw
        .get()
        .as_ptr()
        .addr()
        .checked_sub(source.input.as_ptr().addr())
        .ok_or_else(|| {
            ParseError::new(
                ErrorKind::MalformedInput,
                None,
                "invalid JSON source location",
            )
        })?;
    let end = start.checked_add(raw.get().len()).ok_or_else(|| {
        ParseError::new(
            ErrorKind::ResourceLimit,
            None,
            "JSON source offset overflow",
        )
    })?;
    source.text(start..end)?;
    Ok(source.span(start..end))
}

fn timestamp_date(timestamp: &str) -> Option<String> {
    let (date, time) = timestamp.split_once('T')?;
    if date.len() != 10 {
        return None;
    }
    let time = time.strip_suffix('Z')?;
    let time = if let Some((time, fraction)) = time.split_once('.') {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        time
    } else {
        time
    };
    let fields: Vec<_> = time.split(':').collect();
    let [hour, minute, second] = fields.as_slice() else {
        return None;
    };
    if !fields
        .iter()
        .all(|field| field.len() == 2 && field.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    let (hour, minute, second) = (
        hour.parse::<u8>().ok()?,
        minute.parse::<u8>().ok()?,
        second.parse::<u8>().ok()?,
    );
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(date.to_owned())
}

fn json_error(error: &serde_json::Error) -> ParseError {
    ParseError::new(
        ErrorKind::MalformedInput,
        Some(error.line()),
        format!("invalid GitHub release JSON: {error}"),
    )
}
