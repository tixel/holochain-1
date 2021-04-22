use crate::*;

use cargo_test_support::git::{self, Repository};
use cargo_test_support::{Project, ProjectBuilder};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile;

pub(crate) enum MockProjectType {
    Lib,
    Bin,
}

pub(crate) struct MockProject {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<String>,
    pub(crate) excluded: bool,
    pub(crate) ty: MockProjectType,
    pub(crate) changelog: Option<String>,
}

pub(crate) struct WorkspaceMocker {
    pub(crate) dir: Option<tempfile::TempDir>,
    pub(crate) projects: HashMap<String, MockProject>,
    pub(crate) workspace_project: Project,
    pub(crate) workspace_repo: git2::Repository,
}

impl WorkspaceMocker {
    pub(crate) fn try_new(
        toplevel_changelog: Option<&str>,
        projects: Vec<MockProject>,
    ) -> Fallible<Self> {
        let (path, dir) = {
            let dir = tempfile::tempdir()?;
            if std::option_env!("KEEP_MOCK_WORKSPACE")
                .map(str::parse::<bool>)
                .map(Result::ok)
                .flatten()
                .unwrap_or_default()
            {
                eprintln!("keeping {:?}", dir.path());
                (dir.into_path(), None)
            } else {
                (dir.path().to_path_buf(), Some(dir))
            }
        };

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

        let project_builder = ProjectBuilder::new(path).file(
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
                    use MockProjectType::{Bin, Lib};

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

        let workspace_project = project_builder.build();

        let workspace_mocker = Self {
            dir,
            projects,
            workspace_repo: git::init(&workspace_project.root()),
            workspace_project,
        };

        workspace_mocker.commit(None);

        Ok(workspace_mocker)
    }

    pub(crate) fn root(&self) -> std::path::PathBuf {
        self.workspace_project.root()
    }

    pub(crate) fn add_or_replace_file(&self, path: &str, content: &str) {
        self.workspace_project.change_file(path, content);
    }

    pub(crate) fn commit(&self, tag: Option<&str>) -> String {
        git::add(&self.workspace_repo);
        let commit = git::commit(&self.workspace_repo).to_string();

        if let Some(tag) = tag {
            git::tag(&self.workspace_repo, &tag);
        }

        commit
    }

    pub(crate) fn head(&self) -> String {
        self.workspace_repo
            .revparse_single("HEAD")
            .expect("revparse HEAD")
            .id()
            .to_string()
    }
}

pub(crate) fn example_workspace_1<'a>() -> Fallible<WorkspaceMocker> {
    use crate::tests::workspace_mocker::{self, MockProject, WorkspaceMocker};

    let members = vec![
        MockProject {
            name: "crate_a".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![
                r#"crate_b = { path = "../crate_b", version = "0.0.1" }"#.to_string(),
                r#"crate_c = { path = "../crate_c", version = "0.0.1" }"#.to_string(),
            ],
            excluded: false,
            ty: workspace_mocker::MockProjectType::Bin,
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
        MockProject {
            name: "crate_b".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: false,
            ty: workspace_mocker::MockProjectType::Lib,
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
        MockProject {
            name: "crate_c".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: false,
            ty: workspace_mocker::MockProjectType::Lib,
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
        MockProject {
            name: "crate_d".to_string(),
            version: "0.0.1".to_string(),
            dependencies: vec![],
            excluded: true,
            ty: workspace_mocker::MockProjectType::Bin,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let workspace_mocker = example_workspace_1().unwrap();
        workspace_mocker.add_or_replace_file(
            "README",
            r#"# Example

                Some changes
            "#,
        );
        let before = workspace_mocker.head();
        let after = workspace_mocker.commit(None);

        assert_ne!(before, after);
        assert_eq!(after, workspace_mocker.head());
    }
}
