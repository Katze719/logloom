use std::num::NonZeroUsize;

use clap::Args;
use logloom::{Changelog, Release, Section};

#[derive(Debug, Args)]
#[command(next_help_heading = "Filters")]
pub struct Selection {
    /// Exact release identifier (repeatable).
    #[arg(long = "release", value_name = "VERSION", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    versions: Vec<String>,
    /// Newest published release by date; fails on missing dates or ties.
    #[arg(long, conflicts_with_all = ["versions", "unreleased", "limit", "first_published"])]
    latest: bool,
    /// First published match in file order, skipping drafts.
    #[arg(long, conflicts_with_all = ["versions", "unreleased", "limit"])]
    first_published: bool,
    /// Pending releases, drafts, or fragments.
    #[arg(long, conflicts_with = "versions")]
    unreleased: bool,
    /// Category name, ignoring ASCII case (repeatable).
    #[arg(long = "category", value_name = "NAME", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    categories: Vec<String>,
    /// Component name, ignoring ASCII case (repeatable).
    #[arg(long = "component", value_name = "NAME", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    components: Vec<String>,
    /// Exact package name (repeatable).
    #[arg(long = "package", value_name = "NAME", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    packages: Vec<String>,
    /// First N matches in file order; N must be positive.
    #[arg(long, value_name = "N", value_parser = positive_limit, allow_negative_numbers = true)]
    limit: Option<NonZeroUsize>,
}

#[derive(Debug)]
pub struct SelectionError {
    pub message: String,
    pub help: &'static str,
    pub line: Option<usize>,
}

fn positive_limit(value: &str) -> Result<NonZeroUsize, &'static str> {
    value
        .parse()
        .map_err(|_| "use a positive whole number, e.g. --limit 3")
}

impl Selection {
    pub const fn is_active(&self) -> bool {
        self.latest
            || self.first_published
            || self.unreleased
            || self.limit.is_some()
            || !self.versions.is_empty()
            || !self.categories.is_empty()
            || !self.components.is_empty()
            || !self.packages.is_empty()
    }

    pub fn apply(&self, changelog: &mut Changelog) -> Result<(), SelectionError> {
        changelog.releases.retain_mut(|release| {
            if !self.matches_release(release) {
                return false;
            }
            if !self.categories.is_empty() || !self.components.is_empty() {
                release
                    .sections
                    .retain(|section| self.matches_section(section));
                return !release.sections.is_empty();
            }
            true
        });
        if self.latest {
            let mut newest = None;
            let mut tied = false;
            for release in &changelog.releases {
                let date = release.date.as_deref().ok_or_else(|| SelectionError {
                    message: format!("cannot choose --latest: release {:?}{} has no date", release.version.as_deref().unwrap_or("unversioned"), release.attributes.get("package").map_or_else(String::new, |package| format!(" ({package})"))),
                    help: "Replace --latest with --first-published for file order, or --release VERSION for a specific release.",
                    line: Some(release.span.line),
                })?;
                if newest.is_none_or(|newest| date > newest) {
                    newest = Some(date);
                    tied = false;
                } else if newest == Some(date) {
                    tied = true;
                }
            }
            if tied {
                let matches: Vec<_> = changelog
                    .releases
                    .iter()
                    .filter(|release| release.date.as_deref() == newest)
                    .collect();
                return Err(SelectionError {
                    message: format!(
                        "cannot choose --latest: {} releases share the newest date ({})",
                        matches.len(),
                        newest.unwrap_or_default()
                    ),
                    help: "Narrow the matches with --package, or replace --latest with --release VERSION or --first-published.",
                    line: matches
                        .first()
                        .map(|release| release.date_span.unwrap_or(release.span).line),
                });
            }
            let newest = newest.map(str::to_owned);
            changelog.releases.retain(|release| release.date == newest);
        } else if self.first_published {
            changelog.releases.truncate(1);
        } else if let Some(limit) = self.limit {
            changelog.releases.truncate(limit.get());
        }
        Ok(())
    }

    fn matches_release(&self, release: &Release) -> bool {
        (!(self.latest || self.first_published) || release.is_published())
            && (!self.unreleased || release.is_pending())
            && (self.versions.is_empty()
                || release
                    .version
                    .as_ref()
                    .is_some_and(|version| self.versions.contains(version)))
            && (self.packages.is_empty()
                || release
                    .package()
                    .is_some_and(|package| self.packages.iter().any(|value| value == package)))
    }

    fn matches_section(&self, section: &Section) -> bool {
        (self.categories.is_empty()
            || section.category.as_ref().is_some_and(|category| {
                self.categories
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(category.as_str()))
            }))
            && (self.components.is_empty()
                || section.component.as_ref().is_some_and(|component| {
                    self.components
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(component))
                }))
    }
}
