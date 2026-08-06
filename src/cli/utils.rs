use crate::ast::ast_types::Path;
use crate::diagnostics::PathCache;
use std::fs;
use std::path::PathBuf;

pub fn ast_path_to_file(path: &Path, cache: &mut PathCache) -> Result<String, std::io::Error> {
    let fpath_str = path.0.join("/") + ".zg";
    let fpath = fpath_str.parse::<PathBuf>().expect("Bad path.");
    let file_str = load_file(&fpath)?;
    cache.insert_source(path, &file_str);
    Ok(file_str)
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

pub fn load_pathbuf(file: &PathBuf) -> Option<(String, String, PathCache)> {
    let text = match load_file(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: could not read `{}`: {e}", file.display());
            return None;
        }
    };
    let mut cache = PathCache::new();
    cache.insert_source(&file.clone().into(), &text);
    Some((file.to_string_lossy().into_owned(), text, cache))
}

pub mod targets {
    use super::*;
    use crate::ast::ast_types::*;
    use crate::ast::*;

    pub fn tokenize(file_str: &str, name: &PathBuf) -> Vec<Token> {
        lexer::tokenize_code(file_str, name)
    }

    pub fn build_ast(
        file_str: &str,
        name: &str,
        cache: &mut PathCache,
    ) -> Result<Node, tree::ParseError> {
        let tokens = lexer::tokenize_code(file_str, &PathBuf::from(name));
        tree::parse_file(tokens, name, cache)
    }
}

pub mod verifiers {
    use super::*;

    pub fn scope_check(
        file_str: &str,
        name: &str,
        cache: &mut PathCache,
    ) -> Result<(), crate::semantics::sem_types::ScopeError> {
        let ast = super::targets::build_ast(file_str, name, cache).expect("Couldn't build AST");
        crate::semantics::scope_checker::populate_scope(&ast)?;
        Ok(())
    }

    /*
    pub fn flow_graph_check(path) -> Result<()h (), crate::semantics::types::> {

    }
    */
}
