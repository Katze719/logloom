use std::io::{self, Write};

use logloom::{Category, Changelog, Entry, EntryKind, Format, ParseOptions, Release, Section};

use super::{Failure, Warnings};
use crate::cli::render::{Buffer, readable};

const PACKAGE: &str = "logloom-placeholder";
const AUTHOR: &str = "Unknown <unknown@example.invalid>";
const DATE: &str = "1970-01-01";

pub fn render(
    document: &Changelog,
    format: Format,
    warnings: &mut Warnings,
) -> Result<Vec<u8>, Failure> {
    let mut output = Buffer::default();
    if document.releases.is_empty() {
        warnings.add("no selected releases: native history output is empty");
        return Ok(output.bytes);
    }
    let plain = matches!(
        format,
        Format::DebianChangelog | Format::GnuNews | Format::GnuChangeLog
    );
    if plain && (document.metadata.title.is_some() || !document.metadata.description.is_empty()) {
        warnings.add("document metadata is retained as prose in the plain-text destination");
    }
    if !plain {
        let title = if format == Format::Changesets {
            document
                .releases
                .first()
                .and_then(Release::package)
                .unwrap_or_else(|| {
                    warnings.add(format!("missing Changesets package: using {PACKAGE:?}"));
                    PACKAGE
                })
        } else {
            document.metadata.title.as_deref().unwrap_or("Changelog")
        };
        let title = safe_title(title);
        writeln!(output, "# {}\n", readable::escaped_inline(&title))?;
    } else if format != Format::DebianChangelog
        && let Some(title) = &document.metadata.title
    {
        writeln!(output, "Document title: {}\n", single_line(title))?;
    }
    if format != Format::DebianChangelog && !document.metadata.description.is_empty() {
        if !plain && description_kind(document) == EntryKind::Text {
            warnings.add("introduction preserved as fenced text to avoid structural headings in the destination");
        }
        let first = document.releases.first().ok_or("missing first release")?;
        if plain {
            let mut buffer = Buffer::default();
            readable::converted_plain(
                document,
                first,
                &document.metadata.description,
                if raw_document(document) {
                    EntryKind::Text
                } else {
                    EntryKind::Markdown
                },
                &mut buffer,
            )?;
            indented(&mut output, std::str::from_utf8(&buffer.bytes)?, "  ")?;
        } else {
            readable::converted_markdown(
                document,
                first,
                &Entry::new(
                    document.metadata.description.clone(),
                    description_kind(document),
                ),
                &mut output,
            )?;
        }
        writeln!(output)?;
    }
    for (index, release) in document.releases.iter().enumerate() {
        warnings.release(
            index,
            "native history normalizes original headings and omits source spans",
        );
        if plain {
            plain_release(document, release, index, format, &mut output, warnings)?;
        } else {
            markdown_release(document, release, index, format, &mut output, warnings)?;
        }
    }
    // Verify that content cannot accidentally swallow a release boundary.
    let text = std::str::from_utf8(&output.bytes)?;
    let parsed = ParseOptions::without_input_limit().parse(text, format)?;
    if parsed.releases.len() != document.releases.len() {
        return Err("target content changed release boundaries; choose a structured destination such as JSON".into());
    }
    Ok(output.bytes)
}

fn markdown_release(
    document: &Changelog,
    release: &Release,
    index: usize,
    format: Format,
    output: &mut Buffer,
    warnings: &mut Warnings,
) -> Result<(), Failure> {
    let label = version(release, index, format, warnings);
    write!(output, "## {}", readable::escaped_inline(&label))?;
    let prose_date = label == "Unreleased"
        && matches!(
            format,
            Format::ConventionalChangelog | Format::GitHubReleaseNotes | Format::Changesets
        );
    if let Some(date) = &release.date
        && !prose_date
    {
        if format == Format::Towncrier || format == Format::ConventionalChangelog {
            write!(output, " ({date})")?;
        } else {
            write!(output, " - {date}")?;
        }
    }
    let prose_yanked = label == "Unreleased"
        && !matches!(format, Format::KeepAChangelog | Format::GenericMarkdown);
    if release.yanked && !prose_yanked {
        write!(output, " [YANKED]")?;
    }
    writeln!(output, "\n")?;
    if release.yanked && prose_yanked {
        writeln!(output, "YANKED\n")?;
        warnings.release(
            index,
            "withdrawn status on pending content is preserved as prose",
        );
    }
    if prose_date && let Some(date) = &release.date {
        writeln!(output, "Original date: {date}\n")?;
        warnings.release(index, "date on unversioned content is preserved as prose to retain a recognizable Unreleased heading");
    }
    if release
        .version
        .as_deref()
        .is_some_and(|original| original != label)
    {
        writeln!(
            output,
            "Original version: {}\n",
            readable::escaped_inline(release.version.as_deref().unwrap_or_default())
        )?;
    }
    for (key, value) in &release.attributes {
        writeln!(
            output,
            "- **{}**: {}",
            readable::escaped_inline(key),
            readable::escaped_inline(value)
        )?;
    }
    if !release.attributes.is_empty() {
        writeln!(output)?;
        warnings.release(
            index,
            "format-specific attributes are preserved as prose rather than structured metadata",
        );
    }
    if let Some(link) = &release.link {
        // Link as content avoids target-specific release-label recognition rules.
        writeln!(output, "Release link: {}\n", readable::escaped_inline(link))?;
        warnings.release(index, "release link is preserved as prose");
    }
    if release.version.is_none() && !release.is_pending() {
        warnings.release(index, "unversioned notes or dated records use an Unreleased heading; their original state is preserved in attributes as prose");
    }
    markdown_sections(document, release, index, format, output, warnings)
}

fn markdown_sections(
    document: &Changelog,
    release: &Release,
    index: usize,
    format: Format,
    output: &mut Buffer,
    warnings: &mut Warnings,
) -> Result<(), Failure> {
    let mut component = None;
    for section in &release.sections {
        let nested = format == Format::KeepAChangelog
            && section.component.as_deref().is_some_and(component_name);
        if nested && section.component.as_deref() != component {
            writeln!(
                output,
                "### {}\n",
                readable::escaped_inline(section.component.as_deref().unwrap_or_default())
            )?;
        }
        let heading = section_label(section, !nested);
        let heading = if format == Format::Towncrier && heading.contains(" (") {
            warnings.release(index, "category parentheses use fullwidth parentheses to avoid a Towncrier release boundary");
            heading.replace('(', "（").replace(')', "）")
        } else {
            heading
        };
        let heading = if starts_release(&heading)
            && format != Format::KeepAChangelog
            && format != Format::GenericMarkdown
            && format != Format::Changesets
        {
            warnings.release(
                index,
                "category heading prefixed with 'Category:' to avoid a release boundary",
            );
            format!("Category: {heading}")
        } else {
            heading
        };
        writeln!(
            output,
            "{} {}\n",
            if nested { "####" } else { "###" },
            readable::escaped_inline(&heading)
        )?;
        if section.component.is_some() && !nested {
            warnings.release(
                index,
                "component and category are combined into one section label",
            );
        }
        if section.category.is_none() {
            warnings.release(index, "unnamed section is labeled Uncategorized");
        }
        if format == Format::Towncrier
            && !matches!(section.category, Some(Category::Custom(_)) | None)
        {
            warnings.release(
                index,
                "Towncrier category vocabulary may normalize category names",
            );
        }
        for entry in &section.entries {
            if entry.kind != EntryKind::Text
                && (raw_document(document) || structural_markdown(&entry.markdown))
            {
                let mut raw = entry.clone();
                raw.kind = EntryKind::Text;
                readable::converted_markdown(document, release, &raw, output)?;
                warnings.release(
                    index,
                    "raw text or structural Markdown is preserved in a fenced text block",
                );
            } else {
                readable::converted_markdown(document, release, entry, output)?;
            }
        }
        component = if nested {
            section.component.as_deref()
        } else {
            None
        };
    }
    Ok(())
}

fn version(release: &Release, index: usize, format: Format, warnings: &mut Warnings) -> String {
    let required = format == Format::DebianChangelog;
    if release.version.is_none() && !required {
        return "Unreleased".into();
    }
    if let Some(version) = &release.version {
        let allowed = !version.is_empty()
            && !version.contains(['\r', '\n', '[', ']'])
            && !version.contains(char::is_whitespace)
            && !version.eq_ignore_ascii_case("unreleased")
            && !version.eq_ignore_ascii_case("next")
            && !version.contains(" - ")
            && !version.contains(" (")
            && (format != Format::GnuNews || !version.contains('?'))
            && (format != Format::DebianChangelog || !version.contains(['(', ')']))
            && (!matches!(
                format,
                Format::ConventionalChangelog
                    | Format::GitHubReleaseNotes
                    | Format::Changesets
                    | Format::Towncrier
            ) || starts_release(version));
        if allowed {
            return version.clone();
        }
    }
    let placeholder = format!("0.0.0-logloom-placeholder-{}", index.saturating_add(1));
    warnings.release(
        index,
        format!(
            "missing or unrepresentable version {:?}: using {placeholder:?}",
            release.version
        ),
    );
    placeholder
}

fn plain_release(
    document: &Changelog,
    release: &Release,
    index: usize,
    format: Format,
    output: &mut Buffer,
    warnings: &mut Warnings,
) -> Result<(), Failure> {
    plain_heading(document, release, index, format, output, warnings)?;
    let indent = if format == Format::GnuChangeLog {
        "\t"
    } else {
        "  "
    };
    for (key, value) in &release.attributes {
        indented(output, &format!("{key}: {value}"), indent)?;
    }
    if !release.attributes.is_empty() {
        warnings.release(
            index,
            "attributes are also preserved as prose; unsupported fields cannot remain structured",
        );
    }
    if let Some(link) = &release.link {
        indented(output, &format!("Release link: {link}"), indent)?;
        warnings.release(index, "release link is preserved as prose");
    }
    if release.yanked {
        indented(output, "YANKED", indent)?;
        warnings.release(index, "withdrawn status is preserved as prose");
    }
    if release.version.is_none() {
        warnings.release(
            index,
            "native plain-text history may not retain the original pending or unversioned state",
        );
    }
    for section in &release.sections {
        let label = single_line(&section_label(section, true));
        if format == Format::GnuNews {
            if news_heading(&label) {
                writeln!(output, "** Category: {label}\n")?;
                warnings.release(
                    index,
                    "category prefixed with 'Category:' to avoid a GNU NEWS release boundary",
                );
            } else {
                writeln!(output, "** {label}\n")?;
            }
        } else if format == Format::DebianChangelog {
            writeln!(output, "  [ {} ]\n", label.replace(['[', ']'], ""))?;
        } else {
            indented(output, &format!("Section: {label}"), indent)?;
        }
        if section.component.is_some() {
            warnings.release(
                index,
                "component and category are combined into one section label",
            );
        }
        if format == Format::GnuChangeLog || section.category.is_none() {
            warnings.release(
                index,
                "section structure is represented as prose or a placeholder label",
            );
        }
        for entry in &section.entries {
            let mut buffer = Buffer::default();
            readable::converted_plain(
                document,
                release,
                &entry.markdown,
                if raw_document(document) {
                    EntryKind::Text
                } else {
                    entry.kind
                },
                &mut buffer,
            )?;
            if indented(output, std::str::from_utf8(&buffer.bytes)?, indent)? {
                warnings.release(index, "structural-looking text lines are prefixed with '| ' to avoid creating target records or sections");
            }
            if entry.kind != EntryKind::Text {
                warnings.release(index, "Markdown formatting is rendered as plain text");
            }
        }
    }
    if format == Format::DebianChangelog {
        let maintainer = attribute(release, "maintainer", AUTHOR, index, warnings);
        let timestamp = debian_timestamp(release, index, warnings)?;
        writeln!(output, " -- {maintainer}  {timestamp}\n")?;
    }
    Ok(())
}

fn date<'a>(release: &'a Release, index: usize, warnings: &mut Warnings) -> &'a str {
    release.date.as_deref().unwrap_or_else(|| {
        warnings.release(index, format!("missing date: using {DATE:?}"));
        DATE
    })
}

fn attribute<'a>(
    release: &'a Release,
    key: &str,
    fallback: &'a str,
    index: usize,
    warnings: &mut Warnings,
) -> &'a str {
    release
        .attributes
        .get(key)
        .map(String::as_str)
        .filter(|value| {
            !value.trim().is_empty()
                && !value.contains(char::is_control)
                && !value.contains(';')
                && value.trim() == *value
                && (key != "package" || !value.contains(char::is_whitespace))
                && (key != "urgency" || !value.contains([',', '=']))
        })
        .unwrap_or_else(|| {
            warnings.release(
                index,
                format!("missing or unrepresentable {key}: using {fallback:?}"),
            );
            fallback
        })
}

fn timestamp(date: &str) -> Result<String, Failure> {
    let mut parts = date.split('-');
    let year: u32 = parts.next().ok_or("missing year")?.parse()?;
    let month: usize = parts.next().ok_or("missing month")?.parse()?;
    let day: u32 = parts.next().ok_or("missing day")?.parse()?;
    let name = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month)
    .ok_or("invalid month")?;
    // Compute the weekday using bounded calendar values; do not invent a clock.
    let adjusted_year = if month < 3 {
        year.saturating_sub(1)
    } else {
        year
    };
    let offset = [0u32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4]
        .get(month.saturating_sub(1))
        .ok_or("invalid month")?;
    let weekday = adjusted_year
        .saturating_add(adjusted_year / 4)
        .saturating_sub(adjusted_year / 100)
        .saturating_add(adjusted_year / 400)
        .saturating_add(*offset)
        .saturating_add(day)
        % 7;
    let weekday = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        .get(usize::try_from(weekday)?)
        .ok_or("invalid weekday")?;
    Ok(format!(
        "{weekday}, {day:02} {name} {year:04} 00:00:00 +0000"
    ))
}

fn indented(output: &mut impl Write, value: &str, indent: &str) -> io::Result<bool> {
    let mut escaped = false;
    for line in value.lines() {
        let text = line.trim_start_matches('*').trim();
        let structural = indent == "  "
            && (news_heading(text) || (text.starts_with('[') && text.ends_with(']')));
        if structural {
            writeln!(output, "{indent}| {line}")?;
            escaped = true;
        } else {
            writeln!(output, "{indent}{line}")?;
        }
    }
    writeln!(output)?;
    Ok(escaped)
}

fn section_label(section: &Section, component: bool) -> String {
    let category = section
        .category
        .as_ref()
        .map_or("Uncategorized", Category::as_str);
    if component && let Some(component) = &section.component {
        format!("{component} / {category}")
    } else {
        category.to_owned()
    }
}

fn component_name(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "added" | "changed" | "deprecated" | "removed" | "fixed" | "security"
    )
}

fn starts_release(value: &str) -> bool {
    let value = value.trim_start_matches('[');
    let value = value.strip_prefix('v').unwrap_or(value);
    value.starts_with(|character: char| character.is_ascii_digit())
        || value.eq_ignore_ascii_case("unreleased")
        || value.eq_ignore_ascii_case("next")
}

fn safe_title(value: &str) -> String {
    if starts_release(value) {
        format!("Changelog: {}", single_line(value))
    } else {
        single_line(value)
    }
}

fn single_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn plain_heading(
    document: &Changelog,
    release: &Release,
    index: usize,
    format: Format,
    output: &mut Buffer,
    warnings: &mut Warnings,
) -> Result<(), Failure> {
    if format == Format::DebianChangelog {
        let package = attribute(release, "package", PACKAGE, index, warnings);
        let distributions = attribute(release, "distributions", "UNRELEASED", index, warnings);
        let urgency = attribute(release, "urgency", "low", index, warnings);
        let label = version(release, index, format, warnings);
        writeln!(
            output,
            "{package} ({label}) {distributions}; urgency={urgency}\n"
        )?;
        if release
            .version
            .as_deref()
            .is_some_and(|original| original != label)
        {
            indented(
                output,
                &format!(
                    "Original version: {}",
                    release.version.as_deref().unwrap_or_default()
                ),
                "  ",
            )?;
        }
        if index == 0 {
            if let Some(title) = &document.metadata.title {
                indented(output, &format!("Document title: {title}"), "  ")?;
            }
            if !document.metadata.description.is_empty() {
                let mut buffer = Buffer::default();
                readable::converted_plain(
                    document,
                    release,
                    &document.metadata.description,
                    EntryKind::Markdown,
                    &mut buffer,
                )?;
                indented(output, std::str::from_utf8(&buffer.bytes)?, "  ")?;
            }
        }
    } else if format == Format::GnuNews {
        let label = if release.version.is_none() {
            "?.?".into()
        } else {
            version(release, index, format, warnings)
        };
        write!(output, "* Noteworthy changes in release {label}")?;
        if let Some(date) = &release.date {
            write!(output, " ({date})")?;
        }
        writeln!(output, "\n")?;
        if release
            .version
            .as_deref()
            .is_some_and(|original| original != label)
        {
            indented(
                output,
                &format!(
                    "Original version: {}",
                    release.version.as_deref().unwrap_or_default()
                ),
                "  ",
            )?;
        }
    } else {
        let date = date(release, index, warnings);
        let author = release
            .attributes
            .get("author")
            .or_else(|| release.attributes.get("maintainer"))
            .filter(|author| !author.trim().is_empty() && !author.contains(['\r', '\n']));
        let author = author.map_or_else(
            || {
                warnings.release(index, format!("missing author: using {AUTHOR:?}"));
                AUTHOR
            },
            String::as_str,
        );
        writeln!(output, "{date}  {author}\n")?;
        if let Some(version) = &release.version {
            indented(output, &format!("Version: {version}"), "\t")?;
            warnings.release(
                index,
                "GNU ChangeLog has date/author records; version is preserved as prose",
            );
        }
    }
    Ok(())
}

fn raw_document(document: &Changelog) -> bool {
    matches!(
        document.format,
        Format::DebianChangelog | Format::GnuNews | Format::GnuChangeLog
    ) || (document.format == Format::Towncrier
        && document
            .releases
            .first()
            .is_some_and(|release| release.heading.contains('\n')))
}

fn description_kind(document: &Changelog) -> EntryKind {
    if raw_document(document) || structural_markdown(&document.metadata.description) {
        EntryKind::Text
    } else {
        EntryKind::Markdown
    }
}

fn structural_markdown(content: &str) -> bool {
    let parsed = ParseOptions::without_input_limit().parse(content, Format::GenericMarkdown);
    parsed.is_err()
        || parsed.is_ok_and(|intro| {
            intro.metadata.title.is_some()
                || intro.releases.iter().any(|release| {
                    !release.heading.is_empty()
                        || release
                            .sections
                            .iter()
                            .any(|section| section.heading.is_some())
                })
        })
}

fn debian_timestamp(
    release: &Release,
    index: usize,
    warnings: &mut Warnings,
) -> Result<String, Failure> {
    if let Some(timestamp) = release.attributes.get("timestamp") {
        let sample = format!("p (v) UNRELEASED; urgency=low\n\n -- {AUTHOR}  {timestamp}\n");
        if !timestamp.contains(['\r', '\n'])
            && ParseOptions::without_input_limit()
                .parse(&sample, Format::DebianChangelog)
                .is_ok_and(|candidate| {
                    logloom::validate(&candidate).is_empty()
                        && candidate.releases.first().is_some_and(|candidate| {
                            release.date.is_none() || candidate.date == release.date
                        })
                })
        {
            return Ok(timestamp.clone());
        }
    }
    let date = date(release, index, warnings);
    warnings.release(
        index,
        "missing or inconsistent timestamp: using the release date at 00:00:00 +0000",
    );
    timestamp(date)
}

fn news_heading(value: &str) -> bool {
    [
        "Noteworthy changes in release ",
        "Changes in version ",
        "Changes in release ",
        "Version ",
    ]
    .iter()
    .any(|prefix| value.trim_start_matches('*').trim().starts_with(prefix))
}
