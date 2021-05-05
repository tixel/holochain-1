use super::*;

use crate::tests::workspace_mocker::example_workspace_1;

#[test]
fn detect_changed_files() {
    let workspace_mocker = example_workspace_1().unwrap();
    workspace_mocker.add_or_replace_file(
        "README",
        r#"# Example

            Some changes
        "#,
    );
    let before = workspace_mocker.head().unwrap();
    let after = workspace_mocker.commit(None);

    let workspace = ReleaseWorkspace::try_new(workspace_mocker.root()).unwrap();

    assert_eq!(
        vec![PathBuf::from(&workspace.root().unwrap()).join("README")],
        changed_files(&workspace.root().unwrap(), &before, &after).unwrap()
    );
}

#[test]
fn workspace_members() {
    let workspace_mocker = example_workspace_1().unwrap();
    let workspace = ReleaseWorkspace::try_new(workspace_mocker.root()).unwrap();

    let result = workspace
        .members()
        .unwrap()
        .into_iter()
        .map(|crt| crt.name().to_owned())
        .collect::<Vec<_>>();

    let expected_result = ["crate_a", "crate_b", "crate_c"]
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

    assert_eq!(expected_result, result);
}

#[test]
fn releasable_crates() {
    let workspace_mocker = example_workspace_1().unwrap();
    let workspace = ReleaseWorkspace::try_new(workspace_mocker.root()).unwrap();

    let members = workspace.members().unwrap();

    let result = super::releasable_crates(members)
        .unwrap()
        .into_iter()
        .map(|index| {
            let crt = members.get(index).unwrap();
            crt.name().to_owned()
        })
        .collect::<Vec<_>>();

    let expected_result = ["crate_a", "crate_b"]
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

    assert_eq!(expected_result, result);
}

#[test]
fn detect_changed_crates() {
    let workspace_mocker = example_workspace_1().unwrap();
    workspace_mocker.add_or_replace_file(
        "README",
        r#"# Example

            Some changes
        "#,
    );
    let before = workspace_mocker.head().unwrap();
    let after = workspace_mocker.commit(None);

    let workspace = ReleaseWorkspace::try_new(workspace_mocker.root()).unwrap();

    assert_eq!(
        vec![PathBuf::from(&workspace.root().unwrap()).join("README")],
        changed_files(&workspace.root().unwrap(), &before, &after).unwrap()
    );
}

#[test]
fn release_selection() {
    let workspace_mocker = example_workspace_1().unwrap();
    let workspace = ReleaseWorkspace::try_new(workspace_mocker.root()).unwrap();

    let selection = workspace
        .release_selection()
        .unwrap()
        .into_iter()
        .map(|c| c.name())
        .collect::<Vec<_>>();
    let expected_selection = vec!["crate_a", "crate_b"];

    assert_eq!(expected_selection, selection);
}
