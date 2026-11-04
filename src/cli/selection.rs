use std::num::NonZeroUsize;

use clap::Args;
use logloom::{Changelog, Release, Section};

#[derive(Debug, Args)]
pub struct Selection {
    /// Select an exact release version (repeat for multiple versions).
    #[arg(long = "release", value_name = "VERSION")]
    versions: Vec<String>,
    /// Keep the published release with the newest date; missing dates or ties are errors.
    #[arg(long, conflicts_with_all = ["versions", "unreleased", "limit", "first_published"])]
    latest: bool,
    /// Keep the first matching published release in file order, skipping drafts.
    #[arg(long, conflicts_with_all = ["versions", "unreleased", "limit"])]
    first_published: bool,
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
    pub fn apply(&self, changelog: &mut Changelog) -> Result<(), &'static str> {
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
                let date = release.date.as_deref().ok_or(
                    "cannot determine the latest release: a matching published release has no date",
                )?;
                if newest.is_none_or(|newest| date > newest) {
                    newest = Some(date);
                    tied = false;
                } else if newest == Some(date) {
                    tied = true;
                }
            }
            if tied {
                return Err(
                    "cannot determine the latest release: multiple matching releases share the newest date",
                );
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

        (!(self.latest || self.first_published) || published)
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
