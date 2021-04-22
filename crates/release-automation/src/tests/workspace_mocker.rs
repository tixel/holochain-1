use crate::*;

use cargo_test_support::ProjectBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile;

pub(crate) enum ProjectType {
    Lib,
    Bin,
}

pub(crate) struct Project {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<String>,
    pub(crate) excluded: bool,
    pub(crate) ty: ProjectType,
    pub(crate) changelog: Option<String>,
}

pub(crate) struct WorkspaceMocker {
    pub(crate) dir: tempfile::TempDir,
    pub(crate) project_builder: ProjectBuilder,
    pub(crate) projects: HashMap<String, Project>,
}

impl WorkspaceMocker {
    pub(crate) fn try_new(
        toplevel_changelog: Option<&str>,
        projects: Vec<Project>,
    ) -> Fallible<Self> {
        let dir = tempfile::tempdir()?;

        let projects = projects
            .into_iter()
            .map(|project| (project.name.clone(), project))
            .collect::<HashMap<_, _>>();

        let excluded = projects.iter().fold(String::new(), |acc, (name, project)| {
            if project.excluded {
                acc + &format!(
                    r#"
                        "crates/{}",
                    "#,
                    name
                )
            } else {
                acc
            }
        });

        let project_builder = ProjectBuilder::new(dir.path().to_path_buf()).file(
            "Cargo.toml",
            &format!(
                r#"
                    [workspace]
                    members = [ "crates/*" ]
                    exclude = [
                        {}
                    ]
                    "#,
                excluded
            ),
        );

        let project_builder = if let Some(toplevel_changelog) = toplevel_changelog {
            project_builder.file("CHANGELOG.md", toplevel_changelog)
        } else {
            project_builder
        };

        let project_builder =
            projects
                .iter()
                .fold(project_builder, |project_builder, (name, project)| {
                    use ProjectType::{Bin, Lib};

                    let dependencies = project
                        .dependencies
                        .iter()
                        .fold(String::new(), |dependencies, dependency| {
                            format!("{}{}\n", dependencies, dependency)
                        });

                    let project_builder = project_builder
                        .file(
                            format!("crates/{}/Cargo.toml", &name),
                            &format!(
                                r#"
                            [project]
                            name = "{}"
                            version = "{}"
                            authors = []

                            [dependencies]
                            {}
                            "#,
                                &name, &project.version, dependencies
                            ),
                        )
                        .file(
                            format!(
                                "crates/{}/src/{}",
                                &name,
                                match &project.ty {
                                    Lib => "lib.rs",
                                    Bin => "main.rs",
                                }
                            ),
                            match &project.ty {
                                Lib => "",
                                Bin => "fn main() {}",
                            },
                        );

                    if let Some(changelog) = &project.changelog {
                        project_builder.file(format!("crates/{}/CHANGELOG.md", &name), &changelog)
                    } else {
                        project_builder
                    }
                });

        // todo: initialize git repository

        Ok(Self {
            dir,
            project_builder,
            projects,
        })
    }

    pub(crate) fn path(&self) -> std::path::PathBuf {
        self.dir.path().to_owned()
    }
}

pub(crate) fn example_workspace_1<'a>() -> Fallible<WorkspaceMocker> {
    use crate::tests::workspace_mocker::{self, Project, WorkspaceMocker};

    let members = vec![
        Project {
            name: "crate_a".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![
                r#"crate_b = { path = "../crate_b", version = "0.0.1" }"#.to_string(),
                r#"crate_c = { path = "../crate_c", version = "0.0.1" }"#.to_string(),
            ],
            excluded: false,
            ty: workspace_mocker::ProjectType::Bin,
            changelog: Some(
                r#"
            ---
            ---
            # Changelog

            ## [Unreleased]
            Awesome changes!
            "#
                .to_string(),
            ),
        },
        Project {
            name: "crate_b".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: false,
            ty: workspace_mocker::ProjectType::Lib,
            changelog: Some(
                r#"
            ---
            ---
            # Changelog

            ## [Unreleased]
            Awesome changes!
        "#
                .to_string(),
            ),
        },
        Project {
            name: "crate_c".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: false,
            ty: workspace_mocker::ProjectType::Lib,
            changelog: Some(
                r#"---
unreleasable: true
default_unreleasable: true
---
# Changelog

## [Unreleased]
Awesome changes!
        "#
                .to_string(),
            ),
        },
        Project {
            name: "crate_d".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: true,
            ty: workspace_mocker::ProjectType::Bin,
            changelog: None,
        },
    ];

    WorkspaceMocker::try_new(
        Some(
            r#"
        # Changelog
        Nothing here.

        # Unreleased
        Nothing here yet.
        "#,
        ),
        members,
    )
}
