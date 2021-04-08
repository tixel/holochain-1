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

fn print_node<'a>(
    node: &'a comrak::arena_tree::Node<'a, core::cell::RefCell<comrak::nodes::Ast>>,
    options: Option<ComrakOptions>,
) {
    let mut buf = vec![];
    format_commonmark(node, &options.unwrap_or_default(), &mut buf).unwrap();
    println!("{}", String::from_utf8(buf).unwrap())
}

fn recursive_node_fn<'a, F>(
    node: &'a comrak::arena_tree::Node<'a, core::cell::RefCell<comrak::nodes::Ast>>,
    _reverse: bool,
    f: F,
) where
    F: FnOnce(&'a comrak::arena_tree::Node<'a, core::cell::RefCell<comrak::nodes::Ast>>),
{
    f(node);
    for d in node.descendants().skip(1) {
        recursive_detach(d)
    }
}

fn recursive_detach<'a>(
    node: &'a comrak::arena_tree::Node<'a, core::cell::RefCell<comrak::nodes::Ast>>,
) {
    recursive_node_fn(node, false, |n| n.detach());
}

pub fn process_unreleased_strings(
    inputs: &[(&str, String)],
    output_original: &str,
) -> Fallible<String> {
    let mut options = ComrakOptions::default();
    options.parse.smart = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.hardbreaks = true;

    let output_arena = Arena::new();
    let output_root = parse_document(&output_arena, output_original, &options);

    let mut unreleased_node = None;
    let mut remove_other = false;
    let mut topmost_release = None;

    'root: for (i, node) in output_root.children().enumerate() {
        match &node.data.borrow().value {
            &NodeValue::Heading(heading) => {
                print!("[{}] heading at level {}", i, heading.level);

                match (unreleased_node, heading.level) {
                    (Some(_), 1) => {
                        println!(" => arrived at next release section, stopping.");
                        topmost_release = Some(node);
                        break 'root;
                    }
                    (Some(_), _) => {
                        print!(" => detaching");
                        remove_other = true;
                        node.detach();
                    }
                    (None, 1) => {
                        for (j, node_j) in node.descendants().enumerate().skip(1) {
                            if let NodeValue::Text(ref text) = &node_j.data.borrow().value {
                                let text_str = String::from_utf8_lossy(text);
                                print!(" => [{}] found text '{}'", j, text_str);
                                if text_str.to_lowercase().contains("unreleased") {
                                    print!(" => found unreleased section");
                                    unreleased_node = Some(node);
                                    remove_other = false;
                                    break;
                                };
                            }
                        }
                    }
                    (None, _) => {}
                };

                println!("");
            }

            other => {
                print!("[{}] ", i);
                if remove_other {
                    print!("detaching ");
                    node.detach();
                } else {
                    print!("keeping ");
                }

                match other {
                    NodeValue::Text(ref text) => {
                        print!("'{}'", String::from_utf8_lossy(text))
                    }
                    _ => print!("{:?}", other),
                }

                println!("");
            }
        };
    }

    let input_arena = Arena::new();
    // insert the unreleased content into the output file
    if let Some(ref mut _unreleased_node) = unreleased_node {
        for (name, content) in inputs {
            let root = parse_document(&input_arena, content, &options);

            let mut content_unreleased_heading = None;
            let mut content_topmost_release = None;

            for (i, node) in root.children().enumerate() {
                {
                    let children = node.children().collect::<Vec<_>>().len();
                    let descendants = node.descendants().collect::<Vec<_>>().len();
                    let debug = format!("{:#?}", node.data.borrow().value);
                    let ty = debug.split(&['(', ' '][..]).nth(0).unwrap();
                    println!(
                        "[{}/{}] {} with {} child(ren) and {} descendant(s)",
                        name, i, ty, children, descendants
                    );
                }

                match &mut node.data.borrow_mut().value {
                    &mut NodeValue::Heading(heading) => {
                        println!(
                            "[{}/{}] found heading with level {}",
                            name, i, heading.level
                        );
                        // look for the 'unreleased' heading
                        if heading.level == 2
                            && content_unreleased_heading.is_some()
                            && content_topmost_release.is_none()
                        {
                            println!("[{}/{}] found topmost release", name, i);
                            content_topmost_release = Some(node);
                        } else if heading.level == 2 {
                            // `descendants()` starts with the node itself so we skip it
                            let search = node
                                .descendants()
                                // .children()
                                .skip(1)
                                .collect::<Vec<_>>();

                            println!("[{}/{}] searching through {} nodes", name, i, search.len());

                            for (j, node_j) in search
                                .iter()
                                .take_while(|child| {
                                    child.data.try_borrow().is_ok()
                                    // let stop = if let Ok(data) = child.data.try_borrow() {
                                    //     if let NodeValue::Heading(heading) = data.value {
                                    //         heading.level > 2;
                                    //     } else {
                                    //         true
                                    //     }
                                    // } else {
                                    //     false
                                    // };
                                })
                                .enumerate()
                            {
                                match &mut node_j.data.borrow_mut().value {
                                    NodeValue::Heading(heading) if heading.level < 3 => {
                                        println!(
                                            "[{}/{}/{}] arrived at first release heading, stopping.",
                                            name, i, j
                                        );
                                        break;
                                    }
                                    NodeValue::Text(ref mut text) => {
                                        let text_str = String::from_utf8_lossy(text);
                                        if text_str.to_lowercase().contains("unreleased") {
                                            println!(
                                                "[{}/{}/{}] found unreleased heading: {:#?}",
                                                name, i, j, text_str
                                            );
                                            content_unreleased_heading = Some(node);

                                            *text = format!(
                                                "{}",
                                                // TODO: insert link "[{}](crates/{}/CHANGELOG.md#unreleased)",
                                                name,
                                                // name,
                                            )
                                            .as_bytes()
                                            .to_vec();

                                            println!(
                                                "[{}/{}/{}] changed name to {}",
                                                name, i, j, name
                                            );

                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // &mut NodeValue::FrontMatter(ref fm) => {
                    // let fm_str = String::from_utf8(fm.to_vec()).unwrap();
                    // let fm_yaml = yaml_rust::yaml::YamlLoader::load_from_str(&fm_str).unwrap();
                    // println!("[{}/{}] found a YAML front matter: {:#?}", name, i, fm_yaml);
                    // }
                    // pg @ &mut NodeValue::Paragraph => {
                    //     println!("paragraph: {:#?}", pg);
                    // }
                    _ => {}
                };
            }

            let target = match (unreleased_node, topmost_release) {
                (_, Some(node)) => node,
                (Some(node), _) => node,
                _ => panic!("expected at least one set"),
            };

            // add all siblings between here and the next headline and add all their descendants recursively
            let count = content_unreleased_heading
                .unwrap()
                .following_siblings()
                .take_while(|node| !node.same_node(content_topmost_release.unwrap()))
                .inspect(|node| {
                    target.insert_before(node);
                })
                .count();

            println!("added {} items", count);
        }
    }

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
---
# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
The text beneath this heading will be retained which allows adding overarching release notes.

## Something outdated maybe
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
The text beneath this heading will be retained which allows adding overarching release notes.

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
