use std::error::Error;
use std::io::Write;

use logloom::{Category, Changelog, Entry, EntryKind, Format, Release, Section};
use serde::Serialize;

const HEADER: [&str; 14] = [
    "format",
    "release_index",
    "row_type",
    "version",
    "date",
    "yanked",
    "release_heading",
    "release_link",
    "attributes",
    "component",
    "category",
    "section_heading",
    "kind",
    "content",
];

#[derive(Serialize)]
struct CsvRecord<'a> {
    format: Format,
    release_index: usize,
    row_type: &'a str,
    version: Option<&'a str>,
    date: Option<&'a str>,
    yanked: bool,
    release_heading: &'a str,
    release_link: Option<&'a str>,
    attributes: &'a str,
    component: Option<&'a str>,
    category: Option<&'a str>,
    section_heading: Option<&'a str>,
    kind: Option<EntryKind>,
    content: Option<&'a str>,
}

pub fn render(changelog: &Changelog, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(output);
    writer.write_record(HEADER)?;
    for (index, release) in changelog.releases.iter().enumerate() {
        let attributes = serde_json::to_string(&release.attributes)?;
        if release.sections.is_empty() {
            writer.serialize(row(
                changelog.format,
                index,
                release,
                &attributes,
                None,
                None,
            ))?;
        }
        for section in &release.sections {
            if section.entries.is_empty() {
                writer.serialize(row(
                    changelog.format,
                    index,
                    release,
                    &attributes,
                    Some(section),
                    None,
                ))?;
            }
            for entry in &section.entries {
                writer.serialize(row(
                    changelog.format,
                    index,
                    release,
                    &attributes,
                    Some(section),
                    Some(entry),
                ))?;
            }
        }
    }
    writer.flush()?;
    Ok(())
}

fn row<'a>(
    format: Format,
    index: usize,
    release: &'a Release,
    attributes: &'a str,
    section: Option<&'a Section>,
    entry: Option<&'a Entry>,
) -> CsvRecord<'a> {
    CsvRecord {
        format,
        release_index: index,
        row_type: if entry.is_some() {
            "entry"
        } else if section.is_some() {
            "section"
        } else {
            "release"
        },
        version: release.version.as_deref(),
        date: release.date.as_deref(),
        yanked: release.yanked,
        release_heading: &release.heading,
        release_link: release.link.as_deref(),
        attributes,
        component: section.and_then(|section| section.component.as_deref()),
        category: section
            .and_then(|section| section.category.as_ref())
            .map(Category::as_str),
        section_heading: section.and_then(|section| section.heading.as_deref()),
        kind: entry.map(|entry| entry.kind),
        content: entry.map(|entry| entry.markdown.as_str()),
    }
}
