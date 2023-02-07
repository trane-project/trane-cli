//! Command line tool that takes a simple knowledge base course configuration file and builds the
//! course in the current directory.

use anyhow::Result;
use clap::Parser;
use std::{env::current_dir, fs};
use trane::course_builder::knowledge_base_builder::SimpleKnowledgeBaseCourse;

#[derive(Debug, Parser)]
#[clap(name = "trane")]
#[clap(author, version, about, long_about = None)]
pub(crate) struct SimpleBuild {
    #[clap(help = "The path to the simple knowledge course configuration file to use")]
    config_file: String,
}

fn main() -> Result<()> {
    // Parse the command-line arguments.
    let args = SimpleBuild::parse();

    // Parse the input file and build the course.
    let config_path = &current_dir()?.join(args.config_file);
    let simple_course =
        serde_json::from_str::<SimpleKnowledgeBaseCourse>(&fs::read_to_string(config_path)?)?;
    simple_course.build(&current_dir()?)
}
