#![allow(unused_imports)]
#![allow(dead_code)]

// #[macro_use]
// extern crate educe;

use comrak::{format_commonmark, parse_document, Arena, ComrakOptions};

pub(crate) mod changelog;
pub(crate) mod crate_selection;

#[cfg(test)]
pub(crate) mod tests;

type Fallible<T> = anyhow::Result<T>;

pub(crate) mod cli {
    use std::path::PathBuf;
    use structopt::StructOpt;

    #[derive(StructOpt)]
    #[structopt(name = "ra")]
    pub(crate) enum Commands {
        Changelog(Changelog),
    }

    #[derive(StructOpt, Debug)]
    pub(crate) struct Changelog {
        #[structopt(long)]
        /// Input CHANGELOG.md files to be aggregated
        input_paths: Vec<PathBuf>,

        #[structopt(long)]
        /// Output CHANGELOG.md file to update
        output_path: PathBuf,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = <cli::Commands as structopt::StructOpt>::from_args();

    match command {
        cli::Commands::Changelog(cl) => {
            println!("changelog: {:#?}", cl);
        }
    }

    // let root = parse_document(
    //     &arena,
    //     &std::fs::read_to_string("crates/holochain/CHANGELOG.md").unwrap(),
    //     &options,
    // );

    // fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
    // where
    //     F: Fn(&'a AstNode<'a>),
    // {
    //     f(node);
    //     for c in node.children() {
    //         iter_nodes(c, f);
    //     }
    // }

    // // TODO: consolidate all changelogs

    Ok(())
}
