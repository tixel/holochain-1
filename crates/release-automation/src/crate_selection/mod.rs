//! Select which crates to include in the release process.

use crate::changelog::{self, ChangeType, CrateChangelog};
use crate::Fallible;

use anyhow::{anyhow, bail};
use once_cell::unsync::{Lazy, OnceCell};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod aliases {
    pub use cargo::core::package::Package as CargoPackage;
    pub use cargo::core::Workspace as CargoWorkspace;
}
use aliases::*;

#[derive(Debug)]
pub(crate) struct Crate<'a> {
    package: CargoPackage,
    changelog: Option<CrateChangelog<'a>>,
}

impl<'a> Crate<'a> {
    /// Instantiate a new Crate with the given CargoPackage.
    pub(crate) fn with_cargo_package(package: CargoPackage) -> Fallible<Self> {
        let changelog = {
            let changelog_path = package.root().join("CHANGELOG.md");
            if changelog_path.exists() {
                Some(crate::changelog::CrateChangelog::try_from_path(
                    &changelog_path,
                )?)
            } else {
                None
            }
        };

        Ok(Self { package, changelog })
    }

    /// This crate's name as given in the Cargo.toml file
    pub(crate) fn name(&self) -> String {
        self.package.name().to_string()
    }

    /// This crate's changelog.
    pub(crate) fn changelog(&'a self) -> Option<&CrateChangelog<'a>> {
        self.changelog.as_ref()
    }

    /// Returns the crates in the same workspace that this crate depends on.
    pub(crate) fn dependencies_in_workspace(&'a self) -> Fallible<&Crate<'a>> {
        todo!("")
    }

    pub(crate) fn root(&self) -> &Path {
        self.package.root()
    }
}

pub(crate) struct ReleaseWorkspace<'a> {
    root_path: PathBuf,
    cargo_config: cargo::util::config::Config,
    cargo_workspace: OnceCell<CargoWorkspace<'a>>,
    members: OnceCell<Vec<Crate<'a>>>,
}

impl std::fmt::Debug for ReleaseWorkspace<'_> {
    fn fmt(&self, fmter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        indoc::writedoc!(
            fmter,
            r#"
            ReleaseWorkspace {{
                root_path: {:?},
                cargo_config: <omitted>
                cargo_workspace: {:#?}
                members: {:#?}
            }}
            "#,
            self.root_path,
            self.cargo_workspace.get(),
            self.members.get(),
        )?;

        Ok(())
    }
}

impl<'a> ReleaseWorkspace<'a> {
    pub fn try_new(root_path: PathBuf) -> Fallible<ReleaseWorkspace<'a>> {
        let new = Self {
            root_path,
            cargo_config: cargo::util::config::Config::default()?,

            cargo_workspace: Default::default(),
            members: Default::default(),
        };

        // todo(optimization): eagerly ensure that the workspace is valid, but the following fails lifetime checks
        // let _ = new.cargo_workspace()?;

        Ok(new)
    }

    fn cargo_workspace(&'a self) -> Fallible<&'a CargoWorkspace> {
        self.cargo_workspace.get_or_try_init(|| {
            CargoWorkspace::new(&self.root_path.join("Cargo.toml"), &self.cargo_config)
        })
    }

    /// Returns the crates that are going to be processed for release.
    pub(crate) fn release_selection(&'a self) -> Fallible<Vec<&'a Crate>> {
        let members = self.members()?;
        println!(
            "all members: {:#?}",
            members.iter().map(|m| m.name()).collect::<Vec<_>>()
        );

        let changed = changed_crates(self.root()?, members)?;
        let releasable = releasable_crates(members)?;

        let changed_and_unreleasable = changed.difference(&releasable);
        println!(
            "changed and unreleasable crates: {:#?}",
            changed_and_unreleasable
                .map(|i| members[*i].name())
                .collect::<Vec<_>>()
        );

        let changed_and_releasable = changed.intersection(&releasable).collect::<BTreeSet<_>>();
        println!(
            "changed and releasable crates: {:#?}",
            changed_and_releasable
                .iter()
                .map(|i| members[**i].name())
                .collect::<Vec<_>>()
        );

        // todo(backlog): assert that no changed and releasable crate is blocked by having an unreleasable crate in its dependency tree"

        Ok(members
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if changed_and_releasable.contains(&i) {
                    Some(c)
                } else {
                    None
                }
            })
            .collect())
    }

    /// Returns all non-excluded workspace members.
    pub(crate) fn members(&'a self) -> Fallible<&'a Vec<Crate>> {
        self.members.get_or_try_init(|| {
            let mut members = vec![];

            for package in self.cargo_workspace()?.members() {
                members.push(Crate::with_cargo_package(package.to_owned())?);
            }

            // todo: ensure members are ordered respecting their dependency tree

            Ok(members)
        })
    }

    pub(crate) fn root(&'a self) -> Fallible<&Path> {
        Ok(self.cargo_workspace()?.root())
    }
}

/// Filters the result of `Self::members` by crates that don't have `unreleasable = true` in their CHANGELOG.md front matter.
fn releasable_crates<'a, C>(crates: C) -> Fallible<BTreeSet<usize>>
where
    C: std::iter::IntoIterator<Item = &'a Crate<'a>>,
{
    let mut releasable = BTreeSet::new();
    for (index, candidate) in crates.into_iter().enumerate() {
        match candidate.changelog().map(|cl| cl.front_matter()) {
            // front matter found, include if unreleasable is not indicated
            Some(Ok(Some(front_matter))) => {
                if !front_matter.unreleasable() {
                    releasable.insert(index);
                }
            }

            // no front matter
            Some(Ok(None)) => {
                releasable.insert(index);
            }

            // error while getting the front matter
            Some(Err(e)) => {
                use anyhow::Context;
                return Err(e).context(format!(
                    "when parsing front matter of crate '{}'",
                    candidate.name()
                ));
            }

            // no changelog
            None => println!("{} has no changelog, skipping..", candidate.name()),
        }
    }

    Ok(releasable)
}

/// Returns the indices of all crates that changed since its last release.
fn changed_crates<'a, C>(workspace_root: &Path, crates: C) -> Fallible<BTreeSet<usize>>
where
    C: std::iter::IntoIterator<Item = &'a Crate<'a>>,
{
    let git_repo = git2::Repository::open(workspace_root)?;

    let mut changed = BTreeSet::new();

    for (index, candidate) in crates.into_iter().enumerate() {
        let previous_release = candidate
            .changelog()
            .map(changelog::CrateChangelog::changes)
            .map(Result::ok)
            .flatten()
            .iter()
            .flatten()
            .filter_map(|r| {
                if !r.change_type().is_unreleased() {
                    Some(r.title.clone())
                } else {
                    None
                }
            })
            .take(1)
            .next();

        let git_tag = if let Some(ref previous_release) = previous_release {
            // lookup the git tag for the previous release
            git_repo
                // todo: derive the tagname from a function
                .revparse_single(&format!("{}-v{}", candidate.name(), previous_release))
                .ok()
                .map(|obj| obj.id())
                .map(|id| git_repo.find_tag(id).ok())
                .flatten()
                .map(|tag| tag.name().unwrap_or_default().to_owned())
        } else {
            None
        };

        let change_indicator = if let Some(git_tag) = &git_tag {
            let changed_files = changed_files(candidate.package.root(), &git_tag, "HEAD")?;

            if changed_files.len() > 0 {
                Some(true)
            } else {
                None
            }
        } else {
            None
        };

        println!(
            "[{}] previous release: {:?}, git tag: {:?}, change_indicator: {:?}",
            candidate.name(),
            previous_release,
            git_tag,
            change_indicator,
        );

        if let Some(true) = change_indicator {
            changed.insert(index);
        };
    }

    Ok(changed)
}

/// Use the `git` shell command to detect changed files in the given directory between the given revisions.
///
/// Inspired by: https://github.com/sunng87/cargo-release/blob/master/src/git.rs
fn changed_files(dir: &Path, from_rev: &str, to_rev: &str) -> Fallible<Vec<PathBuf>> {
    use bstr::ByteSlice;

    let output = Command::new("git")
        .arg("diff")
        .arg(&format!("{}..{}", from_rev, to_rev))
        .arg("--name-only")
        .arg("--exit-code")
        .arg(".")
        .current_dir(dir)
        .output()?;

    match output.status.code() {
        Some(0) => Ok(Vec::new()),
        Some(1) => {
            let paths = output
                .stdout
                .lines()
                .map(|l| dir.join(l.to_path_lossy()))
                .collect();
            Ok(paths)
        }
        code => Err(anyhow!("git exited with code: {:?}", code)),
    }
}

#[cfg(test)]
pub(crate) mod tests;
