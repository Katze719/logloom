use logloom::{Changelog, EntryKind, ErrorKind, Format, ParseOptions, Release};

use super::{ConversionFormat, Failure, Warnings};

pub fn detect(
    input: &str,
    name: Option<&str>,
    options: ParseOptions,
) -> Result<ConversionFormat, Failure> {
    let trimmed = input.trim_start();
    // JSON documents also happen to be YAML; prefer their explicit wire syntax.
    if trimmed.starts_with('{') {
        if serde_json::from_str::<Changelog>(input).is_ok() {
            return Ok(ConversionFormat::Json);
        }
        if input
            .lines()
            .filter(|line| !line.trim().is_empty())
            .all(|line| serde_json::from_str::<Release>(line).is_ok())
        {
            return Ok(ConversionFormat::Jsonl);
        }
    }
    #[cfg(feature = "csv")]
    if super::table::signature(input) {
        return Ok(ConversionFormat::Csv);
    }
    let candidates: &[Option<ConversionFormat>] = &[
        #[cfg(feature = "yaml")]
        (!trimmed.starts_with(['{', '['])
            && yaml_safe(input).is_ok()
            && yaml_serde::from_str::<Changelog>(input).is_ok())
        .then_some(ConversionFormat::Yaml),
        #[cfg(feature = "toml")]
        toml::from_str::<Changelog>(input)
            .is_ok()
            .then_some(ConversionFormat::Toml),
    ];
    let mut candidates = candidates.iter().flatten();
    match (candidates.next(), candidates.next()) {
        (Some(format), None) => return Ok(*format),
        (None, _) => {}
        _ => return Err("ambiguous structured input; select --input-format explicitly".into()),
    }
    let native = name.map_or_else(
        || options.detect(input),
        |name| options.detect_named(input, name),
    )?;
    ConversionFormat::from_str_name(native.as_str())
}

impl ConversionFormat {
    fn from_str_name(name: &str) -> Result<Self, Failure> {
        <Self as clap::ValueEnum>::from_str(name, false).map_err(Into::into)
    }
}

pub fn read(
    input: &str,
    format: ConversionFormat,
    name: Option<&str>,
    options: ParseOptions,
    warnings: &mut Warnings,
) -> Result<Changelog, Failure> {
    if format == ConversionFormat::Markdown {
        return markdown_view(input, options, warnings);
    }
    if let Some(native) = format.native() {
        return Ok(name.map_or_else(
            || options.parse(input, native),
            |name| options.parse_named(input, native, name),
        )?);
    }
    let document = match format {
        ConversionFormat::Json | ConversionFormat::JsonCompact => serde_json::from_str(input)?,
        #[cfg(feature = "yaml")]
        ConversionFormat::Yaml => {
            yaml_safe(input)?;
            yaml_serde::from_str(input)?
        }
        #[cfg(feature = "toml")]
        ConversionFormat::Toml => toml::from_str(input)?,
        ConversionFormat::Jsonl => {
            let mut document = Changelog::new(Format::GenericMarkdown);
            for (index, line) in input
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.trim().is_empty())
            {
                document.releases.push(
                    serde_json::from_str(line).map_err(|error| {
                        format!("JSONL line {}: {error}", index.saturating_add(1))
                    })?,
                );
            }
            warnings.add("JSONL has no document metadata, source format, or shared reference definitions; these cannot be recovered");
            document
        }
        #[cfg(feature = "csv")]
        ConversionFormat::Csv => super::table::read(input, warnings)?,
        ConversionFormat::Text => super::text::read(input, warnings),
        _ => return Err("select an explicit source format".into()),
    };
    check_markdown(&document)?;
    Ok(document)
}

fn check_markdown(document: &Changelog) -> Result<(), Failure> {
    let options = ParseOptions::without_input_limit();
    for content in std::iter::once(document.metadata.description.as_str()).chain(
        document
            .releases
            .iter()
            .flat_map(|release| &release.sections)
            .flat_map(|section| &section.entries)
            .filter(|entry| entry.kind != EntryKind::Text)
            .map(|entry| entry.markdown.as_str()),
    ) {
        // Generic parsing runs the shared event scanner before recognizing headings.
        // Only its resource check is relevant for an independently imported body.
        if let Err(error) = options.parse(content, Format::GenericMarkdown)
            && error.kind == ErrorKind::ResourceLimit
        {
            return Err(error.into());
        }
    }
    Ok(())
}

pub fn export_warnings(document: &Changelog, format: ConversionFormat, warnings: &mut Warnings) {
    let partial = match format {
        ConversionFormat::Jsonl => true,
        #[cfg(feature = "csv")]
        ConversionFormat::Csv => true,
        _ => false,
    };
    if partial
        && (document.metadata.title.is_some()
            || !document.metadata.description.is_empty()
            || !document.links.is_empty())
    {
        warnings.add("destination omits document metadata and shared reference definitions");
    }
    for (index, release) in document.releases.iter().enumerate() {
        if matches!(format, ConversionFormat::Text | ConversionFormat::Markdown) {
            warnings.release(index, "reading view normalizes headings and omits source spans; original structure may not be recoverable");
        }
        if format == ConversionFormat::Text
            && release
                .sections
                .iter()
                .flat_map(|section| &section.entries)
                .any(|entry| entry.kind != EntryKind::Text)
        {
            warnings.release(index, "Markdown formatting is rendered as plain text");
        }
        #[cfg(feature = "csv")]
        if format == ConversionFormat::Csv {
            warnings.release(index, "CSV omits source spans and local reference definitions; adjacent identical section boundaries may be ambiguous");
        }
    }
}

// Exported YAML needs no anchors. Reject alias expansion before deserializing,
// while allowing stars/ampersands inside quotes and literal/folded text blocks.
#[cfg(feature = "yaml")]
fn yaml_safe(input: &str) -> Result<(), Failure> {
    let mut quote = None;
    let mut block_indent = None;
    for line in input.lines() {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        if let Some(parent) = block_indent {
            if line.trim().is_empty() || indent > parent {
                continue;
            }
            block_indent = None;
        }
        if line.trim_start().starts_with("? ") {
            return Err("YAML explicit mapping keys are unsupported; use the export schema".into());
        }
        let mut escaped = false;
        let mut property = true;
        let mut flow = 0usize;
        let mut tag = false;
        let mut separated = true;
        let mut characters = line.chars().peekable();
        while let Some(character) = characters.next() {
            if escaped {
                escaped = false;
                continue;
            }
            if let Some(delimiter) = quote {
                if character == '\\' && delimiter == '"' {
                    escaped = true;
                } else if character == delimiter {
                    if delimiter == '\'' && characters.peek() == Some(&'\'') {
                        characters.next();
                    } else {
                        quote = None;
                        property = false;
                        separated = false;
                    }
                }
                continue;
            }
            if tag {
                if character.is_whitespace() {
                    tag = false;
                }
                continue;
            }
            if property && character == '!' {
                tag = true;
                continue;
            }
            if separated && character == '#' {
                break;
            }
            if property && matches!(character, '\'' | '"') {
                quote = Some(character);
                continue;
            }
            if property && matches!(character, '&' | '*') {
                return Err(
                    "YAML anchors and aliases are unsupported; expand them before conversion"
                        .into(),
                );
            }
            if property && matches!(character, '|' | '>') {
                block_indent = Some(indent);
            }
            if property && matches!(character, '[' | '{') {
                flow = flow.saturating_add(1);
            } else if flow != 0 && matches!(character, ']' | '}') {
                flow = flow.saturating_sub(1);
                property = false;
            } else if (flow != 0 && character == ',')
                || (character == ':' && characters.peek().is_none_or(|next| next.is_whitespace()))
            {
                property = true;
            } else if property
                && character == '-'
                && characters.peek().is_some_and(|next| next.is_whitespace())
            {
            } else if !character.is_whitespace() {
                property = false;
            }
            separated = character.is_whitespace();
        }
    }
    Ok(())
}

fn markdown_view(
    input: &str,
    options: ParseOptions,
    warnings: &mut Warnings,
) -> Result<Changelog, Failure> {
    warnings.add("Markdown reading-view import reconstructs components and release labels heuristically; original format, attributes rendered as prose, and earlier source spans cannot be recovered");
    let mut document = options
        .parse(input, Format::KeepAChangelog)
        .or_else(|_| options.parse(input, Format::GenericMarkdown))?;
    // The view borrows Keep a Changelog's structure, without adopting its
    // validation conventions or claiming the original document used that format.
    document.format = Format::GenericMarkdown;
    for release in &mut document.releases {
        let state = match release.version.as_deref() {
            Some("Unversioned") => Some(("version-status", "unknown")),
            Some("Changes") if release.date.is_some() => Some(("record-kind", "dated-change")),
            _ => None,
        };
        if let Some((key, value)) = state {
            release.version = None;
            release.attributes.insert(key.into(), value.into());
        }
    }
    Ok(document)
}
