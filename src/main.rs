pub mod ast;
pub mod cli;
pub mod diagnostics;
pub mod ir;
pub mod semantics;
pub mod utils;

fn main() {
    cli::cli::run();
}
