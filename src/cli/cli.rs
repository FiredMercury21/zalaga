use crate::cli::utils::*;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
    Ir,
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
        #[arg(short, long, default_value = "main.zg")]
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: Option<PathBuf>,
        /// Target output type
        #[arg(short, long, value_enum, default_value_t = Target::ELF)]
        target: Target,
    },
    Run {
        /// File to run
        #[arg(short, long, default_value = "main.zg")]
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: Option<PathBuf>,
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
    use Commands::*;

    match Cli::parse().command {
        Build { .. } => println!("Building..."),
        Run { .. } => println!("Running..."),
        Verify { .. } => println!("Verifying..."),
    }
}
