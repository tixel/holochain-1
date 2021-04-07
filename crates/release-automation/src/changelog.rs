use crate::Fallible;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{format_commonmark, parse_document, Arena, ComrakOptions};

// pub fn process_changelogs(inputs: &[(&str, PathBuf)], _output: &PathBuf) -> Fallible<()> {
//     let input_strings = inputs
//     process_changelogs(inputs.map(|input| ), _output: &PathBuf)
//     Ok(())
// }

pub fn sanitize(s: String) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.parse.smart = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.hardbreaks = true;

    let root = parse_document(&arena, &s, &options);
    let mut buf = vec![];
    format_commonmark(root, &options, &mut buf).unwrap();

    String::from_utf8(buf).unwrap()
}

pub fn process_unreleased_strings(
    inputs: &[(&str, String)],
    output_original: &str,
) -> Fallible<String> {
    let mut options = ComrakOptions::default();
    options.parse.smart = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.hardbreaks = true;

    let mut unreleased = vec![];

    for (name, content) in inputs {
        let arena = Arena::new();
        let root = parse_document(&arena, content, &options);

        'second: for (i, node) in root.children().enumerate() {
            println!("[{}/{}]", name, i,);
            match &mut node.data.borrow_mut().value {
                &mut NodeValue::Heading(heading) => {
                    println!("found heading with level {}", heading.level);
                    // look for the 'unreleased' headings
                    if heading.level == 2 {
                        // `descentants()` starts with the node itself so we skip it
                        for (_j, node_j) in node.descendants().enumerate().skip(1) {
                            if let NodeValue::Text(ref text) = &node_j.data.borrow().value {
                                let text_str = String::from_utf8_lossy(text);
                                if text_str.to_lowercase().contains("unreleased") {
                                    println!("[{}] found unreleased heading: {:#?}", i, text_str);

                                    unreleased.push((name, i));
                                    break 'second;
                                }
                            }
                        }
                    }
                }
                &mut NodeValue::Text(ref mut text) => {
                    println!("found text: {}", String::from_utf8_lossy(text));
                    // let orig = std::mem::replace(text, vec![]);
                    // *text = String::from_utf8(orig)
                    //     .unwrap()
                    //     .replace("", "")
                    //     .as_bytes()
                    //     .to_vec();
                }
                &mut NodeValue::FrontMatter(ref fm) => {
                    let fm_str = String::from_utf8(fm.to_vec()).unwrap();
                    let fm_yaml = yaml_rust::yaml::YamlLoader::load_from_str(&fm_str).unwrap();
                    println!("found a YAML front matter: {:#?}", fm_yaml);
                }
                pg @ &mut NodeValue::Paragraph => {
                    println!("paragraph: {:#?}", pg);
                }
                other => {
                    println!("{:#?}", other);
                }
            };
        }
    }

    println!("{:#?}", unreleased);

    // TODO: get the unreleased content for each input string

    let output_arena = Arena::new();
    let output_root = parse_document(&output_arena, output_original, &options);

    let mut unreleased_found = false;
    let mut remove_non_global = false;

    'traversal: for (i, node_edge) in output_root.traverse().enumerate() {
        use comrak::arena_tree::NodeEdge::{End, Start};

        // let visited = std::collections::HashSet::new();
        let mut visited_start: Vec<&comrak::arena_tree::Node<_>> = vec![];
        let mut visited_end: Vec<&comrak::arena_tree::Node<_>> = vec![];

        let node = match node_edge {
            Start(n) => n,
            End(n) => n,
        };

        if visited.iter().any(|existing| existing.same_node(node)) {
            continue;
        } else {
            visited.push(node);
        }

        match &node.data.borrow().value {
            &NodeValue::Heading(heading) => {
                for (j, node_j) in node.descendants().enumerate() {
                    print!("[{}/{}] ", i, j);
                    if let NodeValue::Text(ref text) = &node_j.data.borrow().value {
                        let text_str = String::from_utf8_lossy(text);

                        print!("heading at level {}: '{}'", heading.level, text_str);
                        if unreleased_found {
                            match heading.level {
                                1 => {
                                    println!(" => arrived at next release section, stopping.");
                                    break 'traversal;
                                }
                                2 => {
                                    if text_str.to_lowercase() == "global" {
                                        print!(" => keeping");
                                        remove_non_global = false;
                                    } else {
                                        print!(" => detaching");
                                        remove_non_global = true;
                                        node.detach();
                                        node_j.detach();
                                    }
                                }
                                _ => {}
                            };
                        } else if text_str.to_lowercase().contains("unreleased") {
                            unreleased_found = true;
                        }
                    }
                    println!("");
                }
            }

            other => {
                print!("[{}]", i);
                if remove_non_global {
                    print!(" detaching ");

                    match other {
                        NodeValue::Text(ref text) => {
                            print!("'{}'", String::from_utf8_lossy(text))
                        }
                        _ => print!("{:?}", other),
                    }

                    node.detach();
                }
                println!("");
            }
        };
    }

    // TODO: insert the unreleased content into the output file

    let mut buf = vec![];
    format_commonmark(output_root, &options, &mut buf)?;
    String::from_utf8(buf).map_err(|e| e.to_string().into())
}

#[cfg(test)]
mod test {
    use comrak::*;

    #[test]
    fn test_frontmatter() {
        const INPUT: &str = r#"---
# values: skip|dry-run|run
mode: skip
# values: skip|major|minor|patch|prerelease
bump-version: skip
# values: current|next
publish-version: current
---
# Changelog

## Unreleased
"#;

        let mut options = ComrakOptions::default();
        options.parse.smart = true;
        options.extension.front_matter_delimiter = Some("---".to_owned());
        options.render.hardbreaks = false;
        let arena = Arena::new();
        let root = parse_document(&arena, INPUT, &options);
        let mut buf = Vec::new();
        format_commonmark(&root, &options, &mut buf).unwrap();
        assert_eq!(&String::from_utf8(buf).unwrap(), INPUT);
    }

    #[test]
    fn changelog_aggregation() {
        const INPUTS: &[(&str, &str)] = &[
            (
                "holochain_zome_types",
                r#"---
# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## _**[Unreleased]**_

### Changed
- `Signature` is a 64 byte 'secure primitive'

## 0.0.2-alpha.1

[Unreleased]: https://github.com/holochain/holochain/holochain_zome_types-v0.0.2-alpha.1...HEAD
"#,
            ),
            (
                "holochain",
                r#"---
# values: skip|dry-run|run
mode: skip
# values: skip|major|minor|patch|prerelease
bump-version: skip
# values: current|next
publish-version: skip
---
# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

*Note: Versions 0.0.52-alpha2 and older are part belong to previous iterations of the Holochain architecture and are not tracked here.*

## Unreleased

### Added

- `InstallAppBundle` command added to admin conductor API. [#665](https://github.com/holochain/holochain/pull/665)
- `DnaSource` in conductor_api `RegisterDna` call now can take a `DnaBundle` [#665](https://github.com/holochain/holochain/pull/665)

### Removed

- BREAKING:  `InstallAppDnaPayload` in admin conductor API `InstallApp` command now only accepts a hash.  Both properties and path have been removed as per deprecation warning.  Use either `RegisterDna` or `InstallAppBundle` instead. [#665](https://github.com/holochain/holochain/pull/665)
- BREAKING: `DnaSource(Path)` in conductor_api `RegisterDna` call now must point to `DnaBundle` as created by `hc dna pack` not a `DnaFile` created by `dna_util` [#665](https://github.com/holochain/holochain/pull/665)

## 0.0.100

This is the first version number for the version of Holochain with a refactored state model (you may see references to it as Holochain RSM).
"#,
            ),
        ];

        const OUTPUT_ORIGINAL: &str = r#"
# Changelog
This file conveniently consolidates all of the crates individual CHANGELOG.md files and groups them by timestamps at which crates were released.
The file is updated every time one or more crates are released.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]
This release will be used to test the release automation tooling.

## Global
This heading name is reserved for manual changes in the toplevel changelog and won't be touched.

## Not global
This will be removed.

## [holochain](crates/holochain/CHANGELOG.md#unreleased)
### Added

- `InstallAppBundle` command added to admin conductor API. [#665](https://github.com/holochain/holochain/pull/665)

# [20210304.120604]
This will include the hdk-0.0.100 release.

## [hdk-0.0.100](crates/hdk/CHANGELOG.md#0.0.100)

### Changed
- hdk: fixup the autogenerated hdk documentation.
"#;

        const OUTPUT_FINAL_EXPECTED: &str = r#"
# Changelog
This file conveniently consolidates all of the crates individual CHANGELOG.md files and groups them by timestamps at which crates were released.
The file is updated every time one or more crates are released.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]
This release will be used to test the release automation tooling.

## Global
This heading name is reserved for manual changes in the toplevel changelog and won't be touched.

## [holochain_zome_types](crates/holochain_zome_types/CHANGELOG.md#unreleased)

### Changed
- `Signature` is a 64 byte 'secure primitive'

## [holochain](crates/holochain/CHANGELOG.md#unreleased)

### Added

- `InstallAppBundle` command added to admin conductor API. [#665](https://github.com/holochain/holochain/pull/665)
- `DnaSource` in conductor_api `RegisterDna` call now can take a `DnaBundle` [#665](https://github.com/holochain/holochain/pull/665)

### Removed

- BREAKING:  `InstallAppDnaPayload` in admin conductor API `InstallApp` command now only accepts a hash.  Both properties and path have been removed as per deprecation warning.  Use either `RegisterDna` or `InstallAppBundle` instead. [#665](https://github.com/holochain/holochain/pull/665)
- BREAKING: `DnaSource(Path)` in conductor_api `RegisterDna` call now must point to `DnaBundle` as created by `hc dna pack` not a `DnaFile` created by `dna_util` [#665](https://github.com/holochain/holochain/pull/665)

# [20210304.120604]
This will include the hdk-0.0.100 release.

## [hdk-0.0.100](crates/hdk/CHANGELOG.md#0.0.100)

### Changed
- hdk: fixup the autogenerated hdk documentation.
"#;

        use crate::changelog::sanitize;

        let inputs_sanitized = INPUTS
            .into_iter()
            .map(|(name, input)| (*name, sanitize(input.to_string())))
            .collect::<Vec<_>>();

        let result = crate::changelog::process_unreleased_strings(
            inputs_sanitized.as_slice(),
            OUTPUT_ORIGINAL,
        )
        .unwrap();

        let output_final_expected_sanitized = sanitize(OUTPUT_FINAL_EXPECTED.to_string());

        assert_eq!(
            result,
            output_final_expected_sanitized,
            "{}",
            prettydiff::text::diff_lines(&result, &output_final_expected_sanitized).format()
        );
    }
}
