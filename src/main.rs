pub mod ast;
pub mod cli;
pub mod ir;
pub mod semantics;
pub mod utils;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn main() {
    cli::cli::run();
}
