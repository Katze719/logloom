pub mod changesets;
pub mod debian;
pub mod github;
pub mod gnu;
pub mod histories;
pub mod keep_a_changelog;
pub mod markdown;
pub mod source;
pub mod text;
pub mod towncrier;

use crate::{Changelog, Entry, Metadata, Release, Section, Span};

pub fn document(input: &str) -> Changelog {
    Changelog {
        format: crate::Format::default(),
        metadata: Metadata {
            title: None,
            description: String::new(),
        },
        releases: Vec::new(),
        links: Vec::new(),
        source: input.to_owned(),
    }
}

pub const fn release(version: Option<String>, heading: String, span: Span) -> Release {
    Release {
        version,
        date: None,
        date_span: None,
        yanked: false,
        heading,
        link: None,
        sections: Vec::new(),
        span,
        attributes: std::collections::BTreeMap::new(),
    }
}

pub fn push_entry(release: &mut Release, entry: Entry) {
    if release.sections.is_empty() {
        release.sections.push(Section {
            component: None,
            category: None,
            heading: None,
            entries: Vec::new(),
            span: entry.span,
        });
    }
    if let Some(section) = release.sections.last_mut() {
        section.entries.push(entry);
    }
}
