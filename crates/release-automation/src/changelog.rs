use crate::Fallible;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{format_commonmark, parse_document, Arena, ComrakOptions};
use std::path::PathBuf;

pub fn process_changelogs(inputs: &[(&str, PathBuf)], output: &PathBuf) -> Fallible<()> {
    todo!("aggregate {:?} into {:?}", inputs, output);
}

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
    F: Fn(&'a comrak::arena_tree::Node<'a, core::cell::RefCell<comrak::nodes::Ast>>),
{
    f(node);
    for d in node.children().skip(1) {
        f(d)
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
                            let search = node.descendants().skip(1).collect::<Vec<_>>();

                            println!("[{}/{}] searching through {} nodes", name, i, search.len());

                            let mut recent_link_index = None;

                            for (j, node_j) in search
                                .iter()
                                .take_while(|child| child.data.try_borrow().is_ok())
                                .enumerate()
                            {
                                match &mut node_j.data.borrow_mut().value {
                                    NodeValue::Link(ref mut link) => {
                                        println!("[{}/{}/{}] found link {:#?}", name, i, j, link);
                                        recent_link_index = Some(j);
                                    }

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

                                            *text = format!("{}", name,).as_bytes().to_vec();

                                            println!(
                                                "[{}/{}/{}] changed name to {}",
                                                name, i, j, name
                                            );

                                            let url =
                                                format!("crates/{}/CHANGELOG.md#unreleased", name);

                                            if let Some(link_index) = recent_link_index {
                                                if let NodeValue::Link(ref mut link) =
                                                    search[link_index].data.borrow_mut().value
                                                {
                                                    link.url = url.as_bytes().to_vec();
                                                    println!(
                                                        "[{}/{}/{}] changing link to: {:#?}",
                                                        name, i, j, url
                                                    );
                                                }
                                            } else {
                                                let link_value =
                                                    NodeValue::Link(comrak::nodes::NodeLink {
                                                        url: url.as_bytes().to_vec(),
                                                        title: Default::default(),
                                                    });
                                                let ast = comrak::nodes::Ast::new(link_value);
                                                let link = output_arena.alloc(
                                                    comrak::arena_tree::Node::new(
                                                        core::cell::RefCell::new(ast),
                                                    ),
                                                );
                                                // insert the link node before the text node
                                                node_j.insert_before(link);

                                                // attach the text node as a child of the link
                                                node_j.detach();
                                                link.append(node_j);
                                            }

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
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/fixtures/changelog_aggregation/crates/holochain_zome_types/CHANGELOG.md"
                )),
            ),
            (
                "holochain",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/fixtures/changelog_aggregation/crates/holochain/CHANGELOG.md"
                )),
            ),
        ];

        const OUTPUT_ORIGINAL: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/fixtures/changelog_aggregation/CHANGELOG.md"
        ));

        const OUTPUT_FINAL_EXPECTED: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/fixtures/changelog_aggregation/CHANGELOG_expected.md"
        ));

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
