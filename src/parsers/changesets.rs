use std::collections::HashSet;

use crate::{Changelog, ErrorKind, Format, MAX_INPUT_BYTES, ParseError};

use super::{histories, markdown, source::Source, text};

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    if input
        .lines()
        .next()
        .is_some_and(|line| line.trim() == "---")
    {
        fragment(input)
    } else {
        histories::parse(input, Format::Changesets)
    }
}

fn fragment(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let mut lines = source.lines();
    lines.next();
    let mut packages = Vec::new();
    let mut seen = HashSet::new();
    let mut body_start = None;
    for (span, line) in lines {
        if line.trim() == "---" {
            body_start = Some(span.end);
            break;
        }
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (package, bump) = package_line(line).ok_or_else(|| {
            ParseError::new(
                ErrorKind::MalformedInput,
                Some(span.line),
                "expected a flat YAML package-to-major/minor/patch mapping",
            )
        })?;
        if !seen.insert(package.clone()) {
            return Err(ParseError::new(
                ErrorKind::MalformedInput,
                Some(span.line),
                "duplicate package in changeset frontmatter",
            ));
        }
        packages.push((package, bump, span));
    }
    let body_start = body_start.ok_or_else(|| {
        ParseError::new(
            ErrorKind::MalformedInput,
            Some(1),
            "changeset YAML frontmatter has no closing '---'",
        )
    })?;
    let body = source.text(body_start..input.len())?;
    let expanded = body.len().checked_mul(packages.len().max(1));
    if expanded.is_none_or(|size| size > MAX_INPUT_BYTES) {
        return Err(ParseError::new(
            ErrorKind::ResourceLimit,
            Some(1),
            "expanded changeset summaries exceed the 16 MiB limit",
        ));
    }
    let mut changelog = markdown::body(body)?;
    text::rebase(&mut changelog, &source, body_start)?;
    source
        .text(0..body_start)?
        .trim()
        .clone_into(&mut changelog.metadata.description);
    let template = changelog.releases.pop().ok_or_else(|| {
        ParseError::new(
            ErrorKind::MalformedInput,
            None,
            "missing changeset summary container",
        )
    })?;
    if packages.is_empty() {
        packages.push((String::new(), String::new(), source.span(0..body_start)));
    }
    for (package, bump, span) in packages {
        let mut release = template.clone();
        release.attributes.remove("version-status");
        release
            .attributes
            .insert("fragment".to_owned(), "changesets".to_owned());
        if !package.is_empty() {
            release.attributes.insert("package".to_owned(), package);
            release.attributes.insert("bump".to_owned(), bump);
        }
        source
            .text(text::range(span))?
            .trim()
            .clone_into(&mut release.heading);
        release.span = span;
        changelog.releases.push(release);
    }
    Ok(changelog)
}

fn package_line(line: &str) -> Option<(String, String)> {
    let (key, value) = line.rsplit_once(':')?;
    let key = scalar(key.trim())?;
    if key.is_empty()
        || key.contains(char::is_whitespace)
        || key.starts_with(['&', '*', '{', '[', '!', '#'])
    {
        return None;
    }
    let value = value
        .split_once(" #")
        .map_or(value, |(value, _)| value)
        .trim();
    let bump = scalar(value)?;
    if !matches!(bump.as_str(), "major" | "minor" | "patch") {
        return None;
    }
    Some((key, bump))
}

fn scalar(value: &str) -> Option<String> {
    if value.starts_with('"') {
        serde_json::from_str(value).ok()
    } else if value.starts_with('\'') {
        Some(
            value
                .strip_prefix('\'')?
                .strip_suffix('\'')?
                .replace("''", "'"),
        )
    } else if value.contains(['\'', '"', '{', '}', '[', ']']) {
        None
    } else {
        Some(value.to_owned())
    }
}
