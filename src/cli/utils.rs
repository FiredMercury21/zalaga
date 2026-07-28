use std::fs;
use std::path::PathBuf;

pub fn ast_path_to_file(path: Vec<String>) -> Result<String, std::io::Error> {
    let fpath_str = path.join("/") + ".zg";
    let fpath = fpath_str.parse::<PathBuf>().expect("Bad path.");
    load_file(&fpath)
}

pub fn load_file(path: &PathBuf) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}

pub fn save_to_file(path: &PathBuf, file_str: &str, force: bool) -> Result<(), std::io::Error> {
    if !force {
        if path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "file already exists",
            ));
        }
    }
    fs::write(path, file_str)
}

pub mod targets {
    use crate::ast::ast_types::*;
    use crate::ast::*;

    pub fn tokenize(file_str: &str) -> Vec<Token> {
        lexer::tokenize_code(file_str)
    }

    pub fn build_ast(file_str: &str, name: &str) -> Result<Node, tree::ParseError> {
        let tokens = lexer::tokenize_code(file_str);
        tree::parse_file(tokens, name)
    }
}

pub mod verifiers {
    pub fn scope_check(
        file_str: &str,
        name: &str,
    ) -> Result<(), crate::semantics::types::ScopeError> {
        let ast = super::targets::build_ast(file_str, name).expect("Couldn't build AST");
        crate::semantics::scope_checker::populate_scope(&ast)?;
        Ok(())
    }

    /*
    pub fn flow_graph_check(path) -> Result<()h (), crate::semantics::types::> {

    }
    */
}
