use super::*;

use crate::tests::workspace_mocker::example_workspace_1;

#[test]
fn detect_changed_files() {
    let workspace_path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/tests/fixtures/example_workspace"
    ));

    assert_eq!(
        vec![PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tests/fixtures/example_workspace/crates/holochain/CHANGELOG.md"
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
fn workspace_members() {
    let workspace_mocker = example_workspace_1().unwrap();
    let project = workspace_mocker.project_builder.build();
    let workspace = ReleaseWorkspace::try_new(project.root()).unwrap();

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
    let project = workspace_mocker.project_builder.build();
    let workspace = ReleaseWorkspace::try_new(project.root()).unwrap();

    // eprintln!("created workspace {:#?}", workspace);
    // std::thread::sleep(std::time::Duration::new(30, 0));
    // workspace.cargo_workspace().unwrap();

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
fn final_selection() {
    let workspace_mocker = example_workspace_1().unwrap();
    let project = workspace_mocker.project_builder.build();
    let workspace = ReleaseWorkspace::try_new(project.root()).unwrap();

    let selection = workspace
        .final_selection()
        .unwrap()
        .into_iter()
        .map(|c| c.name())
        .collect::<Vec<_>>();
    let expected_selection = vec!["crate_a", "crate_b"];

    assert_eq!(expected_selection, selection);
}
