use std::io::Write;

use logloom::{Changelog, EntryKind, ErrorKind, Format, ParseOptions, Release, Section};

use super::Failure;
use crate::cli::render::{Buffer, readable};

// A source-format round trip should keep the model, not turn its links and
// unnamed sections into new prose. Verify the reconstructed document before
// accepting this path; edited or foreign fields can require the general writer.
pub fn render(document: &Changelog) -> Result<Option<Vec<u8>>, Failure> {
    if document.releases.is_empty()
        || document.releases.iter().any(|release| {
            !release.attributes.is_empty()
                || !release.links.is_empty()
                || release
                    .sections
                    .iter()
                    .flat_map(|section| &section.entries)
                    .any(|entry| entry.kind == EntryKind::Text)
        })
    {
        return Ok(None);
    }
    for original_headings in [true, false] {
        let bytes = write_document(document, original_headings)?;
        match ParseOptions::without_input_limit()
            .parse(std::str::from_utf8(&bytes)?, Format::KeepAChangelog)
        {
            Ok(actual) if equivalent(document, &actual) => return Ok(Some(bytes)),
            Err(error) if error.kind == ErrorKind::ResourceLimit => return Err(error.into()),
            _ => {}
        }
    }
    Ok(None)
}

fn write_document(document: &Changelog, original_headings: bool) -> Result<Vec<u8>, Failure> {
    let mut output = Buffer::default();
    if let Some(title) = &document.metadata.title {
        writeln!(output, "# {}\n", readable::escaped_inline(title))?;
    }
    if !document.metadata.description.is_empty() {
        writeln!(output, "{}\n", document.metadata.description)?;
    }
    for release in &document.releases {
        release_heading(release, original_headings, &mut output)?;
        let mut component = None;
        for section in &release.sections {
            if let Some(name) = &section.component
                && (component != Some(name.as_str()) || section.category.is_none())
            {
                writeln!(output, "### {}\n", readable::escaped_inline(name))?;
            }
            if let Some(category) = &section.category {
                if original_headings && let Some(heading) = &section.heading {
                    writeln!(output, "{heading}\n")?;
                } else {
                    writeln!(
                        output,
                        "{} {}\n",
                        if section.component.is_some() {
                            "####"
                        } else {
                            "###"
                        },
                        readable::escaped_inline(category.as_str())
                    )?;
                }
            }
            entries(section, &mut output)?;
            component = section.component.as_deref();
        }
    }
    readable::reference_definitions(&document.links, &mut output)?;
    Ok(output.bytes)
}

fn release_heading(release: &Release, original: bool, output: &mut Buffer) -> Result<(), Failure> {
    if original && !release.heading.is_empty() {
        writeln!(output, "{}\n", release.heading)?;
        return Ok(());
    }
    let version = release.version.as_deref().unwrap_or("Unreleased");
    write!(output, "## ")?;
    if let Some(url) = &release.link {
        readable::heading_link(version, url, output)?;
    } else {
        write!(output, "[{}]", readable::escaped_inline(version))?;
    }
    if let Some(date) = &release.date {
        write!(output, " - {date}")?;
    }
    if release.yanked {
        write!(output, " [YANKED]")?;
    }
    writeln!(output, "\n")?;
    Ok(())
}

fn entries(section: &Section, output: &mut Buffer) -> Result<(), Failure> {
    let mut previous = None;
    for entry in &section.entries {
        if previous
            .is_some_and(|kind| kind != EntryKind::ListItem || entry.kind != EntryKind::ListItem)
        {
            writeln!(output)?;
        }
        writeln!(output, "{}", entry.markdown)?;
        previous = Some(entry.kind);
    }
    if previous.is_some() {
        writeln!(output)?;
    }
    Ok(())
}

fn equivalent(expected: &Changelog, actual: &Changelog) -> bool {
    expected.metadata == actual.metadata
        && expected.links.len() == actual.links.len()
        && expected
            .links
            .iter()
            .zip(&actual.links)
            .all(|(expected, actual)| {
                expected.label == actual.label
                    && expected.destination == actual.destination
                    && expected.title == actual.title
            })
        && expected.releases.len() == actual.releases.len()
        && expected
            .releases
            .iter()
            .zip(&actual.releases)
            .all(|(expected, actual)| {
                expected.version == actual.version
                    && expected.date == actual.date
                    && expected.yanked == actual.yanked
                    && expected.link == actual.link
                    && expected.sections.len() == actual.sections.len()
                    && expected
                        .sections
                        .iter()
                        .zip(&actual.sections)
                        .all(|(expected, actual)| {
                            expected.component == actual.component
                                && expected.category == actual.category
                                && expected.entries.len() == actual.entries.len()
                                && expected.entries.iter().zip(&actual.entries).all(
                                    |(expected, actual)| {
                                        expected.kind == actual.kind
                                            && expected.markdown == actual.markdown
                                    },
                                )
                        })
            })
}
