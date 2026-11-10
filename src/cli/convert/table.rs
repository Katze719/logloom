use std::collections::{BTreeMap, HashMap};

use logloom::{Category, Changelog, Entry, EntryKind, Format, Release, Section};
use serde::Deserialize;

use super::{Failure, Warnings};

#[derive(Deserialize)]
struct Record {
    format: Format,
    release_index: usize,
    row_type: String,
    version: Option<String>,
    date: Option<String>,
    yanked: bool,
    #[serde(rename = "release_heading")]
    heading: String,
    #[serde(rename = "release_link")]
    link: Option<String>,
    attributes: String,
    component: Option<String>,
    category: Option<String>,
    section_heading: Option<String>,
    kind: Option<EntryKind>,
    content: Option<String>,
}

pub fn signature(input: &str) -> bool {
    let mut reader = csv::Reader::from_reader(input.as_bytes());
    reader.headers().is_ok_and(|header| {
        header.iter().eq([
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
        ])
    })
}

pub fn read(input: &str, warnings: &mut Warnings) -> Result<Changelog, Failure> {
    if !signature(input) {
        return Err("CSV header must match logloom's 14-column export schema".into());
    }
    let mut document = Changelog::new(Format::GenericMarkdown);
    let mut indices = HashMap::new();
    let mut reader = csv::Reader::from_reader(input.as_bytes());
    for row in reader.deserialize::<Record>() {
        let row = row?;
        let attributes: BTreeMap<String, String> = serde_json::from_str(&row.attributes)?;
        let position = if let Some(position) = indices.get(&row.release_index) {
            *position
        } else {
            let position = document.releases.len();
            if position == 0 {
                document.format = row.format;
            }
            indices.insert(row.release_index, position);
            let mut release = Release::new(row.version.clone());
            release.date.clone_from(&row.date);
            release.yanked = row.yanked;
            release.heading.clone_from(&row.heading);
            release.link.clone_from(&row.link);
            release.attributes.clone_from(&attributes);
            document.releases.push(release);
            position
        };
        if document.format != row.format {
            return Err("CSV rows disagree on source format".into());
        }
        let release = document
            .releases
            .get_mut(position)
            .ok_or("invalid CSV release index")?;
        if release.version != row.version
            || release.date != row.date
            || release.yanked != row.yanked
            || release.heading != row.heading
            || release.link != row.link
            || release.attributes != attributes
        {
            return Err("CSV rows disagree on metadata for the same release_index".into());
        }
        match row.row_type.as_str() {
            "release" => {
                if row.component.is_some()
                    || row.category.is_some()
                    || row.section_heading.is_some()
                    || row.kind.is_some()
                    || row.content.is_some()
                {
                    return Err("CSV release row contains section or entry values".into());
                }
            }
            "section" | "entry" => {
                let category = row.category.as_deref().map(category);
                let same = release.sections.last().is_some_and(|section| {
                    section.component == row.component
                        && section.category == category
                        && section.heading == row.section_heading
                        && !section.entries.is_empty()
                });
                if row.row_type == "section" || !same {
                    let mut section = Section::new(category);
                    section.component = row.component;
                    section.heading = row.section_heading;
                    release.sections.push(section);
                } else {
                    warnings.release(position, "adjacent CSV rows with identical section labels may have belonged to separate sections; their boundary cannot be recovered");
                }
                if row.row_type == "entry" {
                    let kind = row.kind.ok_or("CSV entry row is missing kind")?;
                    let content = row.content.unwrap_or_default();
                    release
                        .sections
                        .last_mut()
                        .ok_or("missing CSV section")?
                        .entries
                        .push(Entry::new(content, kind));
                } else if row.kind.is_some() || row.content.is_some() {
                    return Err("CSV empty section row contains entry values".into());
                }
            }
            _ => return Err("unknown CSV row_type; expected release, section, or entry".into()),
        }
    }
    warnings.add("CSV has no document metadata, reference definitions, or source spans; these cannot be recovered");
    Ok(document)
}

pub fn category(value: &str) -> Category {
    match value {
        "Added" => Category::Added,
        "Changed" => Category::Changed,
        "Deprecated" => Category::Deprecated,
        "Removed" => Category::Removed,
        "Fixed" => Category::Fixed,
        "Security" => Category::Security,
        custom => Category::Custom(custom.to_owned()),
    }
}
