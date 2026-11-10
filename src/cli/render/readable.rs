use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::ops::Range;

use logloom::{Category, Changelog, EntryKind, LinkDefinition, Release};
use pulldown_cmark::{BrokenLink, CowStr, Event, LinkType, Options, Parser, Tag, TagEnd};
use unicase::UniCase;

use super::release_label;

pub fn markdown(changelog: &Changelog, output: &mut impl Write) -> io::Result<()> {
    writeln!(
        output,
        "# {}\n",
        inline(changelog.metadata.title.as_deref().unwrap_or("Changelog"))
    )?;
    block(output, &changelog.metadata.description)?;
    for release in &changelog.releases {
        let local_links = (!release.links.is_empty())
            .then(|| references(changelog.links.iter().chain(&release.links)));
        write!(output, "## ")?;
        let label = inline(release_label(release));
        if let Some(link) = &release.link {
            write!(output, "[{label}](<{}>)", destination(link))?;
        } else {
            write!(output, "{label}")?;
        }
        if let Some(date) = &release.date {
            write!(output, " - {}", inline(date))?;
        }
        if release.yanked {
            write!(output, " [YANKED]")?;
        }
        writeln!(output, "\n")?;
        let mut attributes = visible_attributes(release).peekable();
        if attributes.peek().is_some() {
            for (key, value) in attributes {
                writeln!(output, "- **{}**: {}", inline(key), inline(value))?;
            }
            writeln!(output)?;
        }
        let mut component = None;
        for section in &release.sections {
            let next = section.component.as_deref();
            if next != component {
                if let Some(name) = next {
                    writeln!(output, "### {}\n", inline(name))?;
                }
                component = next;
            }
            let level = if component.is_some() { "####" } else { "###" };
            let category = section
                .category
                .as_ref()
                .map_or("Uncategorized", Category::as_str);
            writeln!(output, "{level} {}\n", inline(category))?;
            for entry in &section.entries {
                if entry.kind == EntryKind::Text {
                    let fence = fence(&entry.markdown);
                    writeln!(
                        output,
                        "{fence}text\n{}\n{fence}\n",
                        entry.markdown.trim_end_matches(['\r', '\n'])
                    )?;
                } else if let Some(links) = &local_links {
                    scoped_block(output, &entry.markdown, links)?;
                } else {
                    block(output, &entry.markdown)?;
                }
            }
        }
    }
    for link in &changelog.links {
        write!(
            output,
            "[{}]: <{}>",
            label(&link.label),
            destination(&link.destination)
        )?;
        if let Some(title) = &link.title {
            write!(
                output,
                " \"{}\"",
                title
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace(['\r', '\n'], " ")
            )?;
        }
        writeln!(output)?;
    }
    Ok(())
}

pub fn text(changelog: &Changelog, output: &mut impl Write) -> io::Result<()> {
    let links = references(&changelog.links);
    writeln!(
        output,
        "{}\n",
        changelog.metadata.title.as_deref().unwrap_or("Changelog")
    )?;
    if !changelog.metadata.description.is_empty() {
        plain(&changelog.metadata.description, &links, output)?;
        writeln!(output)?;
    }
    for release in &changelog.releases {
        let local_links = (!release.links.is_empty())
            .then(|| references(changelog.links.iter().chain(&release.links)));
        let release_links = local_links.as_ref().unwrap_or(&links);
        write!(output, "{}", release_label(release))?;
        if let Some(date) = &release.date {
            write!(output, " - {date}")?;
        }
        if release.yanked {
            write!(output, " [YANKED]")?;
        }
        if let Some(link) = &release.link {
            write!(output, " <{link}>")?;
        }
        writeln!(output)?;
        for (key, value) in visible_attributes(release) {
            writeln!(output, "{key}: {value}")?;
        }
        writeln!(output)?;
        for section in &release.sections {
            if let Some(component) = &section.component {
                write!(output, "{component} / ")?;
            }
            writeln!(
                output,
                "{}",
                section
                    .category
                    .as_ref()
                    .map_or("Uncategorized", Category::as_str)
            )?;
            for entry in &section.entries {
                if entry.kind == EntryKind::Text {
                    writeln!(output, "{}", entry.markdown)?;
                } else {
                    plain(&entry.markdown, release_links, output)?;
                }
                writeln!(output)?;
            }
        }
    }
    if !changelog.links.is_empty() {
        writeln!(output, "Links:")?;
        for link in &changelog.links {
            writeln!(output, "{}: {}", link.label, link.destination)?;
        }
    }
    Ok(())
}

fn visible_attributes(release: &Release) -> impl Iterator<Item = (&str, &str)> {
    release
        .attributes
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "record-kind" | "version-status"))
        .map(|(key, value)| (key.as_str(), value.as_str()))
}

type References<'a> = HashMap<UniCase<String>, (&'a str, &'a str)>;

fn references<'a>(links: impl IntoIterator<Item = &'a LinkDefinition>) -> References<'a> {
    links
        .into_iter()
        .map(|link| {
            (
                normalized(&link.label),
                (
                    link.destination.as_str(),
                    link.title.as_deref().unwrap_or(""),
                ),
            )
        })
        .collect()
}

// Convert only reference suffixes, using the event stream to leave inline code,
// escaped brackets, formatting, and nested image/link labels intact. Local
// definitions cannot be appended to the combined document without collisions.
fn scoped_block(output: &mut impl Write, value: &str, links: &References<'_>) -> io::Result<()> {
    let value = value.trim_end_matches(['\r', '\n']);
    if value.is_empty() {
        return Ok(());
    }
    let callback = |link: BrokenLink<'_>| {
        links
            .get(&normalized(&link.reference))
            .map(|(url, title)| (CowStr::Borrowed(url), CowStr::Borrowed(title)))
    };
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_with_broken_link_callback(value, options, Some(callback));
    let mut edits = Vec::new();
    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(
            Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }
            | Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            },
        ) = event
            && let Some(suffix) = reference_suffix(value, range, link_type)?
        {
            edits.push((suffix, dest_url, title));
        }
    }
    edits.sort_by_key(|(range, ..)| range.start);
    let mut cursor = 0;
    for (range, url, title) in edits {
        write_range(output, value, cursor..range.start)?;
        write!(output, "(<{}>", destination(&url))?;
        if !title.is_empty() {
            write!(output, " \"{}\"", escaped_title(&title))?;
        }
        write!(output, ")")?;
        cursor = range.end;
    }
    write_range(output, value, cursor..value.len())?;
    writeln!(output, "\n")
}

fn reference_suffix(
    value: &str,
    range: Range<usize>,
    kind: LinkType,
) -> io::Result<Option<Range<usize>>> {
    match kind {
        LinkType::Shortcut | LinkType::ShortcutUnknown => Ok(Some(range.end..range.end)),
        LinkType::Collapsed | LinkType::CollapsedUnknown => {
            let start = range.end.checked_sub(2).ok_or_else(invalid_range)?;
            Ok(Some(start..range.end))
        }
        LinkType::Reference | LinkType::ReferenceUnknown => {
            let text = value.get(range.clone()).ok_or_else(invalid_range)?;
            let mut escaped = false;
            let mut last_open = None;
            for (index, character) in text.char_indices() {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '[' {
                    last_open = Some(index);
                }
            }
            let start = last_open
                .and_then(|offset| range.start.checked_add(offset))
                .ok_or_else(invalid_range)?;
            Ok(Some(start..range.end))
        }
        _ => Ok(None),
    }
}

fn invalid_range() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid Markdown source range")
}

fn write_range(output: &mut impl Write, value: &str, range: Range<usize>) -> io::Result<()> {
    output.write_all(value.get(range).ok_or_else(invalid_range)?.as_bytes())
}

fn escaped_title(title: &str) -> String {
    title
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ")
}

fn normalized(label: &str) -> UniCase<String> {
    UniCase::new(label.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn plain(input: &str, links: &References<'_>, output: &mut impl Write) -> io::Result<()> {
    let callback = |link: BrokenLink<'_>| {
        links
            .get(&normalized(&link.reference))
            .map(|(url, title)| (CowStr::Borrowed(url), CowStr::Borrowed(title)))
    };
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_with_broken_link_callback(input, options, Some(callback));
    let mut writer = Plain {
        output,
        newline: true,
    };
    let mut depth: usize = 0;
    let mut destinations = Vec::new();
    for event in parser {
        match event {
            Event::Text(value)
            | Event::Code(value)
            | Event::Html(value)
            | Event::InlineHtml(value) => writer.write(&value)?,
            Event::Start(Tag::List(_)) => depth = depth.saturating_add(1),
            Event::Start(Tag::Item) => {
                writer.line()?;
                for _ in 1..depth {
                    writer.write("  ")?;
                }
                writer.write("- ")?;
            }
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                destinations.push(dest_url);
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                if let Some(url) = destinations.pop() {
                    writer.write(" <")?;
                    writer.write(&url)?;
                    writer.write(">")?;
                }
            }
            Event::End(TagEnd::List(_)) => {
                depth = depth.saturating_sub(1);
                writer.line()?;
            }
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(
                TagEnd::Item
                | TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::CodeBlock
                | TagEnd::BlockQuote(_)
                | TagEnd::TableRow,
            ) => writer.line()?,
            Event::End(TagEnd::TableCell) => writer.write("\t")?,
            Event::Rule => {
                writer.line()?;
                writer.write("---")?;
                writer.line()?;
            }
            Event::TaskListMarker(checked) => {
                writer.write(if checked { "[x] " } else { "[ ] " })?;
            }
            _ => {}
        }
    }
    writer.line()
}

struct Plain<'a, W> {
    output: &'a mut W,
    newline: bool,
}

impl<W: Write> Plain<'_, W> {
    fn write(&mut self, value: &str) -> io::Result<()> {
        self.output.write_all(value.as_bytes())?;
        if !value.is_empty() {
            self.newline = value.ends_with('\n');
        }
        Ok(())
    }

    fn line(&mut self) -> io::Result<()> {
        if !self.newline {
            self.write("\n")?;
        }
        Ok(())
    }
}

fn block(output: &mut impl Write, value: &str) -> io::Result<()> {
    if !value.is_empty() {
        writeln!(output, "{}\n", value.trim_end_matches(['\r', '\n']))?;
    }
    Ok(())
}

fn inline(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\r' | '\n' => escaped.push(' '),
            '\\' | '*' | '_' | '[' | ']' | '`' | '#' | '<' | '>' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(['\r', '\n'], " ")
}

fn destination(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        if character.is_ascii_whitespace()
            || character.is_ascii_control()
            || matches!(character, '<' | '>' | '\\')
        {
            // Formatting primitive integers into a String cannot return fmt::Error.
            let _ = write!(escaped, "%{:02X}", u32::from(character));
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn fence(value: &str) -> String {
    let mut longest: usize = 2;
    let mut current: usize = 0;
    for character in value.chars() {
        if character == '`' {
            current = current.saturating_add(1);
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(longest.saturating_add(1))
}

// Conversion writers resolve both document and body-local references before
// combining content into a target that may not support reference definitions.
pub(in crate::cli) fn converted_markdown(
    document: &Changelog,
    release: &Release,
    entry: &logloom::Entry,
    output: &mut impl Write,
) -> io::Result<()> {
    if entry.kind == EntryKind::Text {
        let fence = fence(&entry.markdown);
        writeln!(
            output,
            "{fence}text\n{}\n{fence}\n",
            entry.markdown.trim_end_matches(['\r', '\n'])
        )
    } else {
        scoped_block(
            output,
            &entry.markdown,
            &references(document.links.iter().chain(&release.links)),
        )
    }
}

pub(in crate::cli) fn converted_plain(
    document: &Changelog,
    release: &Release,
    value: &str,
    kind: EntryKind,
    output: &mut impl Write,
) -> io::Result<()> {
    if kind == EntryKind::Text {
        block(output, value)
    } else {
        plain(
            value,
            &references(document.links.iter().chain(&release.links)),
            output,
        )
    }
}

pub(in crate::cli) fn escaped_inline(value: &str) -> String {
    inline(value)
}
