use std::num::NonZeroUsize;

use clap::Args;
use logloom::{Category, Changelog, Release, Section};

#[derive(Debug, Args)]
pub struct Selection {
    /// Select an exact release version (repeat for multiple versions).
    #[arg(long = "version", value_name = "VERSION")]
    versions: Vec<String>,
    /// Keep the first matching published release in file order, skipping drafts.
    #[arg(long, conflicts_with_all = ["versions", "unreleased", "limit"])]
    latest: bool,
    /// Select pending releases, drafts, or fragments.
    #[arg(long, conflicts_with = "versions")]
    unreleased: bool,
    /// Select a normalized category, ignoring ASCII case (repeatable).
    #[arg(long = "category", value_name = "NAME")]
    categories: Vec<String>,
    /// Select a component, ignoring ASCII case (repeatable).
    #[arg(long = "component", value_name = "NAME")]
    components: Vec<String>,
    /// Select an exact package name (repeatable).
    #[arg(long = "package", value_name = "NAME")]
    packages: Vec<String>,
    /// Keep at most N matching releases in file order (N must be positive).
    #[arg(long, value_name = "N")]
    limit: Option<NonZeroUsize>,
}

impl Selection {
    pub fn apply(&self, changelog: &mut Changelog) {
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
            changelog.releases.truncate(1);
        } else if let Some(limit) = self.limit {
            changelog.releases.truncate(limit.get());
        }
    }

    fn matches_release(&self, release: &Release) -> bool {
        let draft = release
            .attributes
            .get("draft")
            .is_some_and(|value| value == "true");
        let dated_record = release
            .attributes
            .get("record-kind")
            .is_some_and(|kind| kind == "dated-change");
        let unknown = release
            .attributes
            .get("version-status")
            .is_some_and(|status| status == "unknown");
        let pending = draft || (release.version.is_none() && !dated_record && !unknown);
        let published = !draft && (release.version.is_some() || dated_record);

        (!self.latest || published)
            && (!self.unreleased || pending)
            && (self.versions.is_empty()
                || release
                    .version
                    .as_ref()
                    .is_some_and(|version| self.versions.contains(version)))
            && (self.packages.is_empty()
                || release
                    .attributes
                    .get("package")
                    .is_some_and(|package| self.packages.contains(package)))
    }

    fn matches_section(&self, section: &Section) -> bool {
        (self.categories.is_empty()
            || section.category.as_ref().is_some_and(|category| {
                category_name(category).is_some_and(|name| {
                    self.categories
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(name))
                })
            }))
            && (self.components.is_empty()
                || section.component.as_ref().is_some_and(|component| {
                    self.components
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(component))
                }))
    }
}

fn category_name(category: &Category) -> Option<&str> {
    match category {
        Category::Added => Some("Added"),
        Category::Changed => Some("Changed"),
        Category::Deprecated => Some("Deprecated"),
        Category::Removed => Some("Removed"),
        Category::Fixed => Some("Fixed"),
        Category::Security => Some("Security"),
        Category::Custom(name) => Some(name),
        _ => None,
    }
}
