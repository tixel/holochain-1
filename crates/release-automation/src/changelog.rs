use crate::Fallible;
use comrak::nodes::Ast;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{format_commonmark, parse_document, Arena, ComrakOptions};
use once_cell::unsync::OnceCell;
use semver::Version;
use serde::Deserialize;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Default, Debug, PartialEq, Deserialize)]
pub(crate) struct Frontmatter {
    unreleasable: Option<bool>,

    default_unreleasable: Option<bool>,
}

impl Frontmatter {
    pub(crate) fn unreleasable(&self) -> bool {
        self.unreleasable.unwrap_or_default()
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct WorkspaceChangelog {}

#[derive(Debug, PartialEq)]
pub(crate) struct WorkspaceReleaseHeading {
    time: SystemTime,
    crates: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum WorkspaceRelease {
    Unreleased,
    Release(WorkspaceReleaseHeading),
}

pub(crate) struct CrateChangelog<'a> {
    path: PathBuf,
    arena: Arena<AstNode<'a>>,
    root: OnceCell<&'a comrak::arena_tree::Node<'a, RefCell<Ast>>>,
}

impl std::fmt::Debug for CrateChangelog<'_> {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "CrateChangelog {{ path: {:?} }}", self.path)?;

        Ok(())
    }
}

impl<'a> CrateChangelog<'a> {
    /// Try to instantiate from parse.
    /// FIXME: Eagerly parse the changelog to raise errors from this fn instead of `Self::root()`.
    pub(crate) fn try_from_path(path: &PathBuf) -> Fallible<Self> {
        let path = path.to_owned();
        let arena = Arena::new();
        let root = { Default::default() };

        Ok(Self { path, arena, root })
    }

    /// Find and parse the frontmatter of this crate's changelog file.
    pub(crate) fn front_matter(&'a self) -> Fallible<Option<Frontmatter>> {
        let root = self.root()?;

        for (i, node) in root.children().enumerate() {
            {
                let children = node.children().collect::<Vec<_>>().len();
                let descendants = node.descendants().collect::<Vec<_>>().len();
                let debug = format!("{:#?}", node.data.borrow().value);
                let ty = debug
                    .split(&['(', ' '][..])
                    .nth(0)
                    .ok_or_else(|| format!("error extracting type from '{}'", debug))
                    .map_err(anyhow::Error::msg)?;
                println!(
                    "[{}] {} with {} child(ren) and {} descendant(s)",
                    i, ty, children, descendants
                );
            }

            match &mut node.data.borrow_mut().value {
                &mut NodeValue::FrontMatter(ref fm) => {
                    let fm_str = String::from_utf8(fm.to_vec())?
                        .replace("---", "")
                        .trim()
                        .to_owned();

                    let fm: Frontmatter = if fm_str.is_empty() {
                        Frontmatter::default()
                    } else {
                        serde_yaml::from_str(&fm_str)?
                    };

                    println!(
                        "[{}] found a YAML front matter: {:#?}\nsource string: \n'{}'",
                        i, fm, fm_str
                    );

                    return Ok(Some(fm));
                }

                // we're only interested in the frontmatter here
                _ => {}
            }
        }

        Ok(None)
    }

    /// Find a list of releases for this crate.
    pub(crate) fn releases(&self) -> Fallible<Vec<WorkspaceRelease>> {
        todo!("")
    }

    fn root(&'a self) -> Fallible<&&'a comrak::arena_tree::Node<'a, RefCell<Ast>>> {
        self.root.get_or_try_init(|| {
            let s = std::fs::read_to_string(&self.path)?;
            let mut options = ComrakOptions::default();
            options.parse.smart = true;
            options.extension.front_matter_delimiter = Some("---".to_owned());
            options.render.hardbreaks = true;
            Ok(parse_document(&self.arena, &s, &options))
        })
    }
}

fn process_unreleased(inputs: &[(&str, PathBuf)], output: &PathBuf) -> Fallible<()> {
    let result = process_unreleased_strings(
        &inputs
            .iter()
            .map(|(name, path)| (*name, std::fs::read_to_string(path).unwrap()))
            .collect::<Vec<_>>(),
        &std::fs::read_to_string(output)?,
    )?;

    let mut output_file = std::fs::File::create(output)?;

    use std::io::Write;
    output_file.write_all(result.as_bytes())?;

    Ok(())
}

fn sanitize(s: String) -> String {
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

fn process_unreleased_strings(
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
                .take_while(|node| {
                    if let Some(content_topmost_release) = content_topmost_release {
                        !node.same_node(content_topmost_release)
                    } else {
                        // todo: handle the case where no previous release was found

                        true
                    }
                })
                .inspect(|node| {
                    target.insert_before(node);
                })
                .count();

            println!("added {} items", count);
        }
    }

    let mut buf = vec![];
    format_commonmark(output_root, &options, &mut buf)?;
    String::from_utf8(buf).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::workspace_mocker::example_workspace_1;
    use comrak::*;

    #[test]
    fn empty_frontmatter() {
        let workspace_mocker = example_workspace_1().unwrap();
        let changelog = CrateChangelog::try_from_path(
            &workspace_mocker.root().join("crates/crate_a/CHANGELOG.md"),
        )
        .unwrap();
        let fm: Result<Option<Frontmatter>, String> =
            changelog.front_matter().map_err(|e| e.to_string());

        assert_eq!(Ok(Some(Frontmatter::default())), fm);
    }

    #[test]
    fn no_frontmatter() {
        let workspace_mocker = example_workspace_1().unwrap();
        let changelog = CrateChangelog::try_from_path(
            &workspace_mocker.root().join("crates/crate_b/CHANGELOG.md"),
        )
        .unwrap();
        let fm: Result<Option<Frontmatter>, String> =
            changelog.front_matter().map_err(|e| e.to_string());

        assert_eq!(Ok(None), fm);
    }

    #[test]
    fn nonempty_frontmatter() {
        let workspace_mocker = example_workspace_1().unwrap();
        let changelog = CrateChangelog::try_from_path(
            &workspace_mocker.root().join("crates/crate_c/CHANGELOG.md"),
        )
        .unwrap();
        let fm: Result<Option<Frontmatter>, String> =
            changelog.front_matter().map_err(|e| e.to_string());

        assert_eq!(
            Ok(Some(Frontmatter {
                unreleasable: Some(true),
                default_unreleasable: Some(true),
            })),
            fm
        );
    }

    #[test]
    fn changelog_aggregation() {
        let workspace_mocker = example_workspace_1().unwrap();

        let inputs: &[(&str, PathBuf)] = &[
            (
                "crate_a",
                workspace_mocker.root().join("crates/crate_a/CHANGELOG.md"),
            ),
            (
                "crate_b",
                workspace_mocker.root().join("crates/crate_b/CHANGELOG.md"),
            ),
            // (
            //     "crate_c",
            //     workspace_mocker.root().join("crates/crate_c/CHANGELOG.md"),
            // ),
        ];
        let output_original = workspace_mocker.root().join("CHANGELOG.md");

        const OUTPUT_FINAL_EXPECTED: &str = indoc::indoc!(
            r#"
            # Changelog
            This file conveniently consolidates all of the crates individual CHANGELOG.md files and groups them by timestamps at which crates were released.
            The file is updated every time one or more crates are released.

            The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
            This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

            # [Unreleased]
            The text beneath this heading will be retained which allows adding overarching release notes.

            ## [crate_a](crates/crate_a/CHANGELOG.md#unreleased)

            ### Added

            - `InstallAppBundle` command added to admin conductor API. [#665](https://github.com/holochain/holochain/pull/665)
            - `DnaSource` in conductor_api `RegisterDna` call now can take a `DnaBundle` [#665](https://github.com/holochain/holochain/pull/665)

            ### Removed

            - BREAKING:  `InstallAppDnaPayload` in admin conductor API `InstallApp` command now only accepts a hash.  Both properties and path have been removed as per deprecation warning.  Use either `RegisterDna` or `InstallAppBundle` instead. [#665](https://github.com/holochain/holochain/pull/665)
            - BREAKING: `DnaSource(Path)` in conductor_api `RegisterDna` call now must point to `DnaBundle` as created by `hc dna pack` not a `DnaFile` created by `dna_util` [#665](https://github.com/holochain/holochain/pull/665)

            ## [crate_b](crates/crate_b/CHANGELOG.md#unreleased)

            ### Changed
            - `Signature` is a 64 byte 'secure primitive'

            # [20210304.120604]
            This will include the hdk-0.0.100 release.

            ## [hdk-0.0.100](crates/hdk/CHANGELOG.md#0.0.100)

            ### Changed
            - hdk: fixup the autogenerated hdk documentation.
            "#
        );

        crate::changelog::process_unreleased(inputs, &output_original).unwrap();
        let result = sanitize(std::fs::read_to_string(output_original).unwrap());

        let output_final_expected_sanitized = sanitize(OUTPUT_FINAL_EXPECTED.to_string());
        assert_eq!(
            result,
            output_final_expected_sanitized,
            "{}",
            prettydiff::text::diff_lines(&result, &output_final_expected_sanitized).format()
        );
    }
}