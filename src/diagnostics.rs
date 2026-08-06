use crate::ast::ast_types::*;
use crate::ast::tree::{ParseError, ParseErrorKind};
//use crate::ir::ir_types::{IRError, IRErrorKind};
use crate::semantics::sem_types::*;
use ariadne::{Label, Report, ReportKind};
use std::collections::{HashMap, HashSet};

/*---File Cache---*/

#[derive(PartialEq, Debug)]
pub struct PathCache {
    sources: HashMap<Path, ariadne::Source<String>>,
    parsing: HashSet<Path>,
}

impl PathCache {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            parsing: HashSet::new(),
        }
    }

    pub fn insert_source(&mut self, path: &Path, contents: &String) {
        self.sources
            .insert(path.clone(), ariadne::Source::from(contents.clone()));
    }

    pub fn insert_parsing(&mut self, path: &Path) {
        self.parsing.insert(path.clone());
    }

    pub fn pop_parsing(&mut self, path: &Path) {
        self.parsing.remove(path);
    }
}

impl ariadne::Cache<Path> for PathCache {
    type Storage = String;

    fn fetch(
        &mut self,
        id: &Path,
    ) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        self.sources
            .get(id)
            .ok_or_else(|| format!("Unknown source: {:?}", id))
    }

    fn display<'a>(&self, id: &'a Path) -> Option<impl std::fmt::Display + 'a> {
        Some(id)
    }
}

pub fn print_report<E: Diagnose>(err: E, cache: &mut PathCache) {
    if let Err(e) = build_report(err).eprint(cache) {
        eprintln!("Error: Could not render error {e}");
    }
}

/*---Report Builder---*/

fn build_report<'a, E: Diagnose>(err: E) -> Report<'a, Span> {
    Report::build(ReportKind::Error, err.span())
        .with_message("An error preventing compilation occurred:")
        .with_label(
            Label::new(err.span())
                .with_message(err.msg())
                .with_color(ariadne::Color::Red),
        )
        .finish()
}

/*---Errors---*/

pub trait Diagnose {
    fn msg(&self) -> String;
    fn span(&self) -> Span;
}

impl Diagnose for ParseError {
    fn msg(&self) -> String {
        use ParseErrorKind::*;

        match &self.err {
            BadExpr { .. } => {
                format!("Incorrect expression syntax")
            }
            BadPath { .. } => {
                format!("Incorrect path syntax")
            }
            BadNum { .. } => {
                format!("Incorrect number")
            }
            BadFloat { .. } => {
                format!("Incorrect floating point number")
            }
            BadPattern { .. } => {
                format!("Incorrect match pattern syntax")
            }
            BadNegation { .. } => {
                format!("Negation on incorrect expression")
            }
            FnNoRetType => {
                format!("Function definition does not have a return type")
            }
            FnNoParen => {
                format!("Function definition does not have a parameter list")
            }
            FnNoName => {
                format!("Function definition does not have a name")
            }
            FnNoBody => {
                format!("Function does not have a properly formatted body")
            }
            FnBadArg { .. } => {
                format!("Incorrect argument in function definition")
            }
            FnSyntax { .. } => {
                format!("Incorrect function syntax")
            }
            FnNoCloseBrack => {
                format!("Argument list never closed (or forgot comma)")
            }
            VarNoType => {
                format!("Variable requires type annotation")
            }
            VarNoName => {
                format!("Variable not given a name")
            }
            VarNoAnnotation => {
                format!("Variable requires type annotation")
            }
            ForNoInit => {
                format!("For loop never declares a variable")
            }
            ForNoPred => {
                format!(
                    "For loop does not have a predicate; If you wish to loop forever, use `true`"
                )
            }
            ForNoBlock => {
                format!("For loop does not have a properly formatted body")
            }
            WhileNoBlock => {
                format!("While loop does not have a properly formatted body")
            }
            AsnBadSyntax { .. } => {
                format!("Incorrect assignment")
            }
            EnumNoBlock => {
                format!("Enum does not have a properly formatted definition body")
            }
            EnumBadSyntax { .. } => {
                format!("Incorrect syntax when defining enum")
            }
            EnumDuplicateVariant { found } => {
                format!("Duplicate variant `{}` found in enum", found)
            }
            StructNoBlock => {
                format!("Struct does not have a properly formatted definition body")
            }
            StructBadSyntax { .. } => {
                format!("Incorrect syntax when defining struct")
            }
            StructNoFieldInit => {
                format!("Did not initialize field when declaring struct literal")
            }
            StructDuplicateField { found } => {
                format!("Duplicate field `{}` found in struct", found)
            }
            BadType { .. } => {
                format!("Incorrect type syntax")
            }
            IfNoBlock => {
                format!("If statement does not have a properly formatted body")
            }
            BlockParseErr { .. } => {
                format!("Error parsing block")
            }
            ExprParseErr { .. } => {
                format!("Error parsing expression")
            }
            UnclosedBrack => {
                format!("Expected closing bracket")
            }
            InvalidKeyword { .. } => {
                format!("Invalid keyword")
            }
            InvalidField { .. } => {
                format!("Incorrect field access")
            }
            ModuleNotFound { found } => {
                format!("Module {} not found", found)
            }
            ErrInModule { path, err } => {
                format!("Parse error in module {}, error: {}", path, err.msg())
            }
            ImportNoName => {
                format!("Attempted importing module without name")
            }
            ImportNoAlias => {
                format!("Alias syntax used, no alias provided for module")
            }
            InvalidSyntax { .. } => {
                format!("Invalid syntax")
            }
            UnexpectedEof => {
                format!("Unexpected end of file")
            }
            EmptyFile => {
                format!("Provided file is empty")
            }
            Generic => {
                format!("Error occured here")
            }
        }
    }

    fn span(&self) -> Span {
        match &self.err {
            ParseErrorKind::ErrInModule { err, .. } => err.span(),
            _ => self.span.clone(),
        }
    }
}

impl Diagnose for ScopeError {
    fn msg(&self) -> String {
        use ScopeErrorKind::*;

        match &self.kind {
            UndefinedVar { path } => {
                format!("Undefined variable: {}", path)
            }
            UndefinedFn { path } => {
                format!("Undefined function: {}", path)
            }
            UndefinedType { path } => {
                format!("Undefined type: {}", path)
            }
            UndefinedField { field } => {
                format!("Undefined field: {}", field)
            }
            UndefinedEnumVariant { parent, name } => {
                format!("Undefined enum variant: {} in enum {}", name, parent)
            }
            AlreadyDeclared { name } => {
                format!("Duplicate definition: {}", name)
            }
            ErrInModule { err, mod_name } => {
                format!("Error in module {}: {}", mod_name, err.msg())
            }
            RecursiveType { ty } => {
                format!("Recursive type definition: {}", ty)
            }
        }
    }

    fn span(&self) -> Span {
        match &self.kind {
            ScopeErrorKind::ErrInModule { err, .. } => err.span(),
            _ => self.span.clone(),
        }
    }
}

impl Diagnose for TypeCheckError {
    fn msg(&self) -> String {
        use TypeCheckErrorKind::*;

        match &self.err {
            FnNotFound { name } => {
                format!("Function not found: {}", name)
            }
            FieldNotFound { name } => {
                format!("Field not found: {}", name)
            }
            TypeMismatch { expected, actual } => {
                format!("Expected type `{}`, but found `{}`", expected.ty, actual.ty)
            }
            VariantOnNonEnum { ty } => {
                format!("Variant on non-enum type: {}", ty.ty)
            }
            VariantNotInEnum { ty, variant } => {
                format!("Variant {} not in enum type {}", variant, ty.ty)
            }
            EnumVariantMissing { expected } => {
                format!("Enum variant missing: {}", expected)
            }
            UnexpectedVariantPattern { patt } => {
                format!("Unexpected variant pattern: {:?}", patt)
            }
            ValOnBlankVariant { ty, variant, val } => {
                format!(
                    "Value {:?} provided for blank variant {} of type {:?}",
                    val, variant, ty
                )
            }
            ValWhenDestructEnum => {
                format!("Value provided when destructuring enum")
            }
            FieldOnNonStruct { ty, field } => {
                format!("Field {} on non-struct type {:?}", field, ty)
            }
            FieldNotInStruct { ty, field } => {
                format!("Field {} not in struct type {:?}", field, ty)
            }
            DerefOnNonRef { ty } => {
                format!("Dereference on non-reference type {:?}", ty)
            }
            ErrInModule { err, mod_name } => {
                format!("Error in module {}: {}", mod_name, err.msg())
            }
            FnArgCountMismatch {
                path,
                expected,
                actual,
            } => {
                format!(
                    "Function {} was provided {} argument{}, expected {}",
                    path,
                    actual,
                    if actual > &1 { "s" } else { "" },
                    expected
                )
            }
        }
    }

    fn span(&self) -> Span {
        match &self.err {
            TypeCheckErrorKind::ErrInModule { err, .. } => err.span(),
            _ => self.span.clone(),
        }
    }
}
/*
implDiagnosefor FlowError {
    fn msg(&self) -> String {
        use FlowErrorKind::*;

        match &self.err {
            UnInitVariable { name } => {
                format!("Uninitialized variable: {}", name)
            }
            InvalidAssignment { name } => {
                format!("Invalid assignment to variable: {}", name)
            }
        }
    }

    fn span(&self) -> Span {
    match &self.kind {
        ScopeErrorKind::ErrInModule { err, .. } => err.span(),
        _ => self.span.clone(),
    }
    }
}

implDiagnosefor IRError {
    fn msg(&self) -> String {
        use IRErrorKind::*;

        match &self.err {
            UndefinedVar { path } => {
                format!("Undefined variable: {}", path)
            }
            UndefinedFn { path } => {
                format!("Undefined function: {}", path)
            }
            UndefinedType { path } => {
                format!("Undefined type: {}", path)
            }
            InvalidType {
                path,
                expected,
                found,
            } => {
                format!(
                    "Invalid type for {}: expected {:?}, found {:?}",
                    path, expected, found
                )
            }
            InvalidFnCall {
                path,
                expected,
                found,
            } => {
                format!(
                    "Invalid function call for {}: expected {:?}, found {:?}",
                    path, expected, found
                )
            }
        }
    }

    fn span(&self) -> Span {
    match &self.kind {
        ScopeErrorKind::ErrInModule { err, .. } => err.span(),
        _ => self.span.clone(),
    }
    }
}
*/
