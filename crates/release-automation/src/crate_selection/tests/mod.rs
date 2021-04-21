use super::*;

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
    // let changed_files = changed_files(&workspace_path, "HEAD", "HEAD");
    let workspace = ReleaseWorkspace::try_new(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/tests/fixtures/example_workspace"
    )))
    .unwrap();

    let result = workspace
        .members()
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
fn releasable_crates() {
    let workspace = ReleaseWorkspace::try_new(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/tests/fixtures/example_workspace"
    )))
    .unwrap();

    let members = workspace.members().unwrap();

    let result = super::releasable_crates(members)
        .unwrap()
        .into_iter()
        .map(|index| {
            let crt = members.get(index).unwrap();
            crt.name().to_owned()
        })
        .collect::<Vec<_>>();

    let expected_result = ["holochain-fixture", "holochain_zome_types-fixture"]
        .iter()
        .map(std::string::ToString::to_string)
        // .map(|name| Crate {
        //     name,
        //     ..Default::default()
        // })
        .collect::<Vec<_>>();

    assert_eq!(expected_result, result);
}

#[test]
fn final_selection() {
    // todo: construct and assert a test case
    // let workspace_mocker = crate::tests::workspace_mocker::WorkspaceMocker::try_new().unwrap();

    let workspace = ReleaseWorkspace::try_new(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/tests/fixtures/example_workspace"
    )))
    .unwrap();

    workspace.final_selection().unwrap();
}
