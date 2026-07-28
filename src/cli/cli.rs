use crate::cli::utils::*;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::cli::utils::load_file;

/// zgc: Compiler and toolchain for Zalaga.
#[derive(Parser)]
#[command(name = "zgc", author, version, about, long_about = None)]
struct Cli {
    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
enum Target {
    Tokens,
    AST,
    IR,
    Asm,
    ELF,
}

#[derive(ValueEnum, Clone, Debug)]
enum Pass {
    Scope,
    Flow,
    Inits,
    Type,
}

impl ToString for Pass {
    fn to_string(&self) -> String {
        match self {
            Pass::Scope => "scope".to_string(),
            Pass::Flow => "flow".to_string(),
            Pass::Inits => "inits".to_string(),
            Pass::Type => "type".to_string(),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Build {
        /// File to compile
        #[arg(short = 'i', long, default_value = "main.zg")]
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
        /// Force write
        #[arg(short, long, default_value_t = false)]
        force: bool,
        /// Target output type
        #[arg(short, long, value_enum, default_value_t = Target::ELF)]
        target: Target,
    },
    Run {
        /// File to run
        #[arg(short = 'i', long, default_value = "main.zg")]
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
        /// Force write
        #[arg(short, long, default_value_t = false)]
        force: bool,

        #[arg(last = true)]
        args: Vec<String>,
    },
    Verify {
        /// File to verify
        #[arg(short, long, default_value = "main.zg")]
        file: PathBuf,
        /// Verification pass.
        #[arg(short, long, default_value_t = Pass::Type)]
        pass: Pass,
    },
}

pub fn run() {
    use super::utils::*;
    use Commands::*;

    match Cli::parse().command {
        Build {
            file,
            output,
            target,
            force,
        } => {
            let file_str = load_file(&file).expect("Bad file.");
            println!("{}", file_str);
            let out_str = match target {
                Target::Tokens => format!("{:#?}", targets::tokenize(&file_str)),
                Target::AST => format!(
                    "{:#?}",
                    targets::build_ast(&file_str, file.to_str().expect("Bad Path"))
                ),
                Target::IR => todo!(),
                Target::Asm => todo!(),
                Target::ELF => todo!(),
            };
            save_to_file(&output, &out_str, force);
        }
        Run { .. } => println!("Running..."),
        Verify { .. } => println!("Verifying..."),
    }
}
