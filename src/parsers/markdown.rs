use std::ops::Range;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    Category, Changelog, Entry, EntryKind, ErrorKind, LinkDefinition, ParseError, Section,
};

use super::{document, push_entry, release, source::Source};

pub struct Heading {
    pub level: HeadingLevel,
    pub text: String,
    pub link: Option<String>,
    pub range: Range<usize>,
}

pub enum Block {
    Heading(Heading),
    Entry(Range<usize>, EntryKind),
}

pub struct Markdown {
    pub blocks: Vec<Block>,
    pub links: Vec<LinkDefinition>,
}

pub fn scan(input: &str) -> Result<Markdown, ParseError> {
    let source = Source::new(input);
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(input, options);
    let mut links: Vec<_> = parser
        .reference_definitions()
        .iter()
        .map(|(label, definition)| LinkDefinition {
            label: label.to_owned(),
            destination: definition.dest.to_string(),
            title: definition.title.as_ref().map(ToString::to_string),
            span: source.span(definition.span.clone()),
        })
        .collect();
    links.sort_by_key(|link| link.span.start);
    let mut blocks = Vec::new();
    let mut stack = Vec::new();
    let mut heading: Option<Heading> = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if stack.len() >= 128 {
                    return Err(ParseError::new(
                        ErrorKind::ResourceLimit,
                        Some(source.span(range).line),
                        "Markdown nesting exceeds 128 containers",
                    ));
                }
                if stack.is_empty() {
                    match &tag {
                        Tag::Heading { level, .. } => {
                            heading = Some(Heading {
                                level: *level,
                                text: String::new(),
                                link: None,
                                range: range.clone(),
                            });
                        }
                        Tag::List(_) => {}
                        _ => blocks.push(Block::Entry(range.clone(), EntryKind::Markdown)),
                    }
                } else if matches!(&tag, Tag::Item) && matches!(stack.as_slice(), [TagEnd::List(_)])
                {
                    blocks.push(Block::Entry(range.clone(), EntryKind::ListItem));
                }
                if let (Some(heading), Tag::Link { dest_url, .. }) = (&mut heading, &tag) {
                    heading.link.get_or_insert_with(|| dest_url.to_string());
                }
                stack.push(tag.to_end());
            }
            Event::End(tag) => {
                stack.pop();
                if stack.is_empty()
                    && matches!(tag, TagEnd::Heading(_))
                    && let Some(heading) = heading.take()
                {
                    blocks.push(Block::Heading(heading));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = &mut heading {
                    heading.text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = &mut heading {
                    heading.text.push(' ');
                }
            }
            Event::Rule if stack.is_empty() => {
                blocks.push(Block::Entry(range, EntryKind::Markdown));
            }
            _ => {}
        }
    }
    Ok(Markdown { blocks, links })
}

pub fn body(input: &str) -> Result<Changelog, ParseError> {
    let source = Source::new(input);
    let markdown = scan(input)?;
    let mut changelog = document(input);
    changelog.links = markdown.links;
    let mut current = release(None, String::new(), source.span(0..input.len()));
    current
        .attributes
        .insert("version-status".to_owned(), "unknown".to_owned());
    for block in markdown.blocks {
        match block {
            Block::Heading(heading) => current.sections.push(Section {
                component: None,
                category: Some(Category::from_name(heading.text.trim())),
                heading: Some(source.text(heading.range.clone())?.trim().to_owned()),
                entries: Vec::new(),
                span: source.span(heading.range),
            }),
            Block::Entry(range, kind) => push_entry(
                &mut current,
                Entry {
                    markdown: source.text(range.clone())?.trim().to_owned(),
                    kind,
                    span: source.span(range),
                },
            ),
        }
    }
    changelog.releases.push(current);
    Ok(changelog)
}
