//! Select which crates to include in the release process.

use crate::changelog::CrateChangelog;
use crate::Fallible;

use anyhow::{anyhow, bail};
use once_cell::unsync::{Lazy, OnceCell};
use std::cell::Cell;
use std::path::PathBuf;
use std::process::Command;

mod aliases {
    pub use cargo::core::package::Package as CargoPackage;
    pub use cargo::core::Workspace as CargoWorkspace;
}
use aliases::*;

struct Crate<'a> {
    // dependencies: Vec<&'a Self>,
    package: CargoPackage,
    changelog: Option<CrateChangelog<'a>>,
}

impl<'a> Crate<'a> {
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

    pub(crate) fn name(&self) -> String {
        self.package.name().to_string()
    }
}

struct ReleaseWorkspace<'a> {
    root_path: PathBuf,
    cargo_config: cargo::util::config::Config,
    cargo_workspace: OnceCell<CargoWorkspace<'a>>,
    crates: OnceCell<Vec<Crate<'a>>>,
}

impl<'a> ReleaseWorkspace<'a> {
    pub fn try_new(root_path: PathBuf) -> Fallible<ReleaseWorkspace<'a>> {
        let new = Self {
            root_path,
            cargo_config: cargo::util::config::Config::default()?,

            cargo_workspace: Default::default(),
            crates: Default::default(),
        };

        // todo: ensure the workspace is valid, but the following fails lifetime checks
        // let _ = new.cargo_workspace()?;

        Ok(new)
    }

    fn cargo_workspace(&'a self) -> Fallible<&'a CargoWorkspace> {
        self.cargo_workspace.get_or_try_init(|| {
            CargoWorkspace::new(&self.root_path.join("Cargo.toml"), &self.cargo_config)
        })
    }

    fn crates(&'a self) -> Fallible<&'a Vec<Crate>> {
        self.crates.get_or_try_init(|| {
            let mut crates = vec![];

            for package in self.cargo_workspace()?.members() {
                crates.push(Crate::with_cargo_package(package.to_owned())?);
            }

            Ok(crates)
        })
    }

    pub fn releasable_crates(&'a mut self) -> Fallible<Vec<Crate>> {
        let releasable_crates = vec![];

        // determine all non-excluded workspace members
        let _crates = self.crates()?;

        // todo: determine which crates have `releasable = false` in their CHANGELOG

        // todo: determine the previous release
        // let changed_paths = changed_files(self.root_path, from_rev: &str, to_rev: &str);

        // todo: determine which crates changed since the most recent release

        // todo: determine whether any release is blocked by an unreleasable crate

        Ok(releasable_crates)
    }
}

// source: https://github.com/sunng87/cargo-release/blob/master/src/git.rs
fn changed_files(dir: &PathBuf, from_rev: &str, to_rev: &str) -> Fallible<Vec<PathBuf>> {
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
mod test {
    use super::*;

    #[test]
    fn detect_changed_files() {
        let workspace_path = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/fixtures/example_workspace"
        ));

        assert_eq!(
            vec![PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/fixtures/example_workspace/crates/holochain/CHANGELOG.md"
            ))],
            changed_files(
                &workspace_path,
                "4470117bfe54bdfadbf4d8a563fd7125742ef9a5",
                "68a31e72d67043acd2037d54396b7eb56ba6ba2e"
            )
            .unwrap()
        );
    }

    #[test]
    fn workspace_crates() {
        // let changed_files = changed_files(&workspace_path, "HEAD", "HEAD");
        let workspace = ReleaseWorkspace::try_new(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/fixtures/example_workspace"
        )))
        .unwrap();

        let result = workspace
            .crates()
            .unwrap()
            .into_iter()
            .map(|crt| crt.name().to_owned())
            .collect::<Vec<_>>();

        let expected_result = [
            "holochain-fixture",
            "holochain_zome_types-fixture",
            "unreleasable",
        ]
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

        assert_eq!(expected_result, result);
    }

    #[test]
    fn workspace_crate_selection() {
        // let changed_files = changed_files(&workspace_path, "HEAD", "HEAD");
        let mut workspace = ReleaseWorkspace::try_new(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/fixtures/example_workspace"
        )))
        .unwrap();

        let result = workspace
            .releasable_crates()
            .unwrap()
            .into_iter()
            .map(|crt| crt.name().to_owned())
            .collect::<Vec<_>>();

        let expected_result = ["holochain", "holochain_zome_types"]
            .iter()
            .map(std::string::ToString::to_string)
            // .map(|name| Crate {
            //     name,
            //     ..Default::default()
            // })
            .collect::<Vec<_>>();

        assert_eq!(expected_result, result);
    }
}
