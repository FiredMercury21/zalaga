use crate::diagnostics::*;
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
    Type,
}

impl std::fmt::Display for Pass {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Pass::Scope => "scope".to_string(),
                Pass::Flow => "flow".to_string(),
                Pass::Type => "type".to_string(),
            }
        )
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
        #[arg(short = 'i', long, default_value = "main.zg")]
        file: PathBuf,
        /// Verification pass.
        #[arg(short, long, default_value_t = Pass::Type)]
        pass: Pass,
    },
}

pub fn run() {
    use super::utils::*;
    use Commands::*;

    fn file_name(path: &PathBuf) -> &str {
        path.to_str().expect("Bad file name.")
    }

    match Cli::parse().command {
        Build {
            file,
            output,
            target,
            force,
        } => {
            let Some((fname, file_str, mut cache)) = load_pathbuf(&file) else {
                return;
            };
            let out_str = match target {
                Target::Tokens => {
                    format!("{:#?}", targets::tokenize(&file_str, &PathBuf::from(fname)))
                }
                Target::AST => format!("{:#?}", {
                    cache.insert_parsing(&(file.clone().into()));
                    let ast = targets::build_ast(&file_str, &fname, &mut cache);
                    cache.pop_parsing(&(file.clone().into()));
                    ast
                }),
                Target::IR => todo!(),
                Target::Asm => todo!(),
                Target::ELF => todo!(),
            };
            if let Err(e) = save_to_file(&output, &out_str, force) {
                eprintln!("Failed to save file, error: {e}");
            };
        }
        Run { .. } => println!("Running..."),
        Verify { file, pass } => {
            use crate::diagnostics::print_report;
            use crate::semantics::*;

            let Some((fname, file_str, mut cache)) = load_pathbuf(&file) else {
                return;
            };
            cache.insert_parsing(&(file.clone().into()));
            let parse_output = targets::build_ast(&file_str, &fname, &mut cache);
            cache.pop_parsing(&(file.clone().into()));
            let ast = match parse_output {
                Ok(root) => root,
                Err(e) => {
                    print_report(e, &mut cache);
                    return;
                }
            };

            match pass {
                Pass::Scope => {
                    if let Err(e) = scope_checker::populate_scope(&ast) {
                        print_report(e, &mut cache);
                        return;
                    };
                }
                Pass::Type => {
                    let mut scope_table = match scope_checker::populate_scope(&ast) {
                        Ok(table) => table,
                        Err(e) => {
                            print_report(e, &mut cache);
                            return;
                        }
                    };
                    if let Err(e) = type_checker::type_check(&ast, &mut scope_table) {
                        print_report(e, &mut cache);
                        return;
                    }
                }
                Pass::Flow => todo!(),
            }
            println!("Pass verified.")
        }
    }
}
