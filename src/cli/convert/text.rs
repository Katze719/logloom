use logloom::{Category, Changelog, Entry, EntryKind, Format, Release, Section};

use super::Warnings;

pub fn read(input: &str, warnings: &mut Warnings) -> Changelog {
    warnings.add("text import reconstructs reading-view headings heuristically; formatting and ambiguous boundaries cannot be recovered");
    let blocks = paragraphs(input);
    let mut document = Changelog::new(Format::GenericMarkdown);
    if !blocks
        .iter()
        .any(|block| block.lines().next().and_then(header).is_some())
    {
        if !input.is_empty() {
            let mut release = Release::new(None);
            release
                .attributes
                .insert("version-status".into(), "unknown".into());
            let mut section = Section::new(None);
            section
                .entries
                .push(Entry::new(input.to_owned(), EntryKind::Text));
            release.sections.push(section);
            document.releases.push(release);
            warnings.release(
                0,
                "no clear release structure; preserved the complete input as unversioned text",
            );
        }
        return document;
    }
    for (index, block) in blocks.iter().enumerate() {
        let mut lines = block.lines();
        let first = lines.next().unwrap_or_default();
        if let Some(mut release) = header(first) {
            for line in lines {
                if let Some((key, value)) = line.split_once(": ") {
                    release.attributes.insert(key.to_owned(), value.to_owned());
                } else {
                    entry(&mut release, line.to_owned());
                }
            }
            document.releases.push(release);
        } else if let Some(release) = document.releases.last_mut() {
            let label = first.split_once(" / ");
            let category = label.map_or(first, |(_, category)| category);
            let known = matches!(
                category,
                "Added"
                    | "Changed"
                    | "Deprecated"
                    | "Removed"
                    | "Fixed"
                    | "Security"
                    | "Uncategorized"
            );
            if known || label.is_some() {
                let mut section =
                    Section::new((category != "Uncategorized").then(|| category_name(category)));
                section.component = label.map(|(component, _)| component.to_owned());
                section.heading = Some(first.to_owned());
                let rest = block
                    .get(first.len()..)
                    .unwrap_or_default()
                    .trim_start_matches(['\r', '\n']);
                if !rest.is_empty() {
                    section
                        .entries
                        .push(Entry::new(rest.to_owned(), EntryKind::Text));
                }
                release.sections.push(section);
            } else {
                entry(release, (*block).to_owned());
            }
        } else if index == 0 && !block.contains('\n') {
            document.metadata.title = Some((*block).to_owned());
        } else {
            if !document.metadata.description.is_empty() {
                document.metadata.description.push_str("\n\n");
            }
            document.metadata.description.push_str(block);
        }
    }
    document
}

fn entry(release: &mut Release, content: String) {
    if release.sections.is_empty() {
        release.sections.push(Section::new(None));
    }
    if let Some(section) = release.sections.last_mut() {
        section.entries.push(Entry::new(content, EntryKind::Text));
    }
}

fn paragraphs(input: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0usize;
    let mut position = 0usize;
    for line in input.split_inclusive('\n') {
        if line.trim().is_empty() {
            if let Some(block) = input
                .get(start..position)
                .map(|block| block.trim_end_matches(['\r', '\n']))
                .filter(|block| !block.is_empty())
            {
                result.push(block);
            }
            start = position.saturating_add(line.len());
        }
        position = position.saturating_add(line.len());
    }
    if let Some(block) = input
        .get(start..)
        .map(|block| block.trim_end_matches(['\r', '\n']))
        .filter(|block| !block.is_empty())
    {
        result.push(block);
    }
    result
}

fn header(heading: &str) -> Option<Release> {
    let (line, destination) = heading
        .rsplit_once(" <")
        .filter(|(_, link)| link.ends_with('>'))
        .map_or((heading, None), |(line, link)| {
            (line, link.strip_suffix('>').map(str::to_owned))
        });
    let (line, yanked) = line
        .strip_suffix(" [YANKED]")
        .map_or((line, false), |line| (line, true));
    let (label, date) = line
        .rsplit_once(" - ")
        .filter(|(_, date)| {
            date.len() == 10
                && date
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'-')
        })
        .map_or((line, None), |(label, date)| (label, Some(date.to_owned())));
    let special = matches!(label, "Unreleased" | "Unversioned" | "Changes");
    let numeric = label
        .strip_prefix('v')
        .unwrap_or(label)
        .starts_with(|character: char| character.is_ascii_digit())
        && !label.contains(char::is_whitespace);
    if !(special || date.is_some() || numeric) {
        return None;
    }
    let mut release = Release::new((!special).then(|| label.to_owned()));
    release.heading = line.to_owned();
    release.date = date;
    release.link = destination;
    release.yanked = yanked;
    if label == "Unversioned" {
        release
            .attributes
            .insert("version-status".into(), "unknown".into());
    }
    if label == "Changes" {
        release
            .attributes
            .insert("record-kind".into(), "dated-change".into());
    }
    Some(release)
}

fn category_name(value: &str) -> Category {
    match value {
        "Added" => Category::Added,
        "Changed" => Category::Changed,
        "Deprecated" => Category::Deprecated,
        "Removed" => Category::Removed,
        "Fixed" => Category::Fixed,
        "Security" => Category::Security,
        value => Category::Custom(value.to_owned()),
    }
}
