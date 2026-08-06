use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Id(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    // Binary Operators
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    Mod,
    Assign,

    // Logical Operators
    LT,
    GT,
    ET,
    LorET,
    GorET,
    NotET,
    Or,
    And,

    // Unary Operators
    Not,
    Neg,
    Inc,
    Dec,
    Ref,
    Deref,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokType {
    // Parens
    LBrack,
    RBrack,
    LSquirl,
    RSquirl,
    LSquare,
    RSquare,

    // Structure
    Indent,
    Dedent,
    Newline,
    Eof,
    Colon,
    SColon,
    Guard,
    Comma,
    Arrow,
    Period,
    At,
    Underscore,
    Separator,

    // Operators
    Op(Operator),

    // Constants
    Num(String),
    Float(String),
    Char(char),

    // Identifiers
    Ident(String),

    Illegal(char),
}

impl std::fmt::Display for TokType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub source: Path,
    pub start: usize,
    pub end: usize,
}

pub trait Spanned {
    fn span(&self) -> Span;
    fn end(&self) -> usize {
        self.span().end
    }
    fn start(&self) -> usize {
        self.span().start
    }
}

// Some Ariadne stuff to make Span work as a Source.

impl ariadne::Span for Span {
    type SourceId = Path;

    fn source(&self) -> &Self::SourceId {
        &self.source
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok_type: TokType,
    pub index: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Num(i64),
    Float(f64),
    Bool(bool),
    Char(char),
}

impl std::fmt::Display for Constant {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        use Constant::*;
        let text = match self {
            Num(n) => &format!("{}", n),
            Float(f) => &format!("{}", f),
            Bool(b) => &format!("{}", b),
            Char(c) => &format!("'{}'", c),
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    All,
    Var { name: String },
    Val { val: Constant },
    Variant { name: String, payload: String },
}

impl std::fmt::Display for Pattern {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        use Pattern::*;
        let text = match self {
            All => "all",
            Var { name } => &format!("{}", name),
            Val { val } => &format!("value: ({})", val),
            Variant { name, payload } => &format!("enum variant: {} -> {}", name, payload),
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub node: NodeKind,
    pub span: Span,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub expr: ExprKind,
    pub span: Span,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Var {
        path: Path,
    },
    Match {
        expr: Box<Expr>,
        grds: Vec<Node>,
    },
    If {
        pred: Box<Expr>,
        then: Box<Expr>,
        else_block: Option<Box<Expr>>,
    },
    Block {
        lines: Vec<Node>,
    },
    FnCall {
        path: Path,
        args: Vec<Expr>, // Maybe make into HashMap, for named args.
    },
    Const {
        val: Constant,
    },
    Field {
        base: Box<Expr>, //Maybe?
        field: String,
    },
    Struct {
        path: Path,
        fields: HashMap<String, Expr>,
    },
    Enum {
        path: Path,
        variant: String,
        val: Option<Box<Expr>>,
    },
    BinOp {
        first: Box<Expr>,
        op: Operator,
        second: Box<Expr>,
    },
    UnOp {
        op: Operator,
        expr: Box<Expr>,
    },
    Return {
        val: Option<Box<Expr>>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Module {
        name: String,
        global: Vec<Node>,
    },
    FnDec {
        name: String,
        args: Vec<Node>,
        ret_type: Box<Node>,
        body: Expr,
    },
    Statement {
        expr: Expr,
    },
    VarDec {
        name: String,
        expr: Option<Expr>,
        var_type: Box<Node>,
    },
    Guard {
        patt: Pattern,
        expr: Expr,
    },
    StructDec {
        name: String,
        fields: Vec<Node>,
    },
    EnumDec {
        name: String,
        variants: Vec<EnumVariant>,
    },
    For {
        init: Box<Node>,
        pred: Expr,
        then: Expr,
        block: Expr,
    },
    While {
        pred: Expr,
        block: Expr,
    },
    Use {
        name: String,
        root: Box<Node>,
    },
    // TODO: Make 'Type' its own type.
    // It kinda already is. Why'd I use a Node??
    Type {
        name: TypeNode,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub var_type: Option<Box<Node>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeNode {
    Ref(Box<TypeNode>),
    Base(Path),
    Infer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Path(pub Vec<String>);

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self(vec![s.to_string()])
    }
}

impl From<&String> for Path {
    fn from(s: &String) -> Self {
        Self(vec![s.clone()])
    }
}

impl From<std::path::PathBuf> for Path {
    fn from(p: std::path::PathBuf) -> Self {
        let mut segments: Vec<String> = p
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        if let Some(last) = segments.last_mut() {
            if let Some(stripped) = last.strip_suffix(".zg") {
                *last = stripped.to_string();
            }
        }
        Self(segments)
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("::"))
    }
}

impl Spanned for Node {
    fn span(&self) -> Span {
        self.span.clone()
    }
}

impl Spanned for Expr {
    fn span(&self) -> Span {
        self.span.clone()
    }
}

impl Spanned for Token {
    fn span(&self) -> Span {
        self.index.clone()
    }
}

impl Path {
    pub fn push(&mut self, name: String) {
        self.0.push(name);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn first(&self) -> &str {
        &self.0[0]
    }

    pub fn pop_first(&mut self) {
        self.0.remove(0);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn base(&self) -> &str {
        &self.0[self.0.len() - 1]
    }

    pub fn fname(&self) -> Option<&str> {
        self.0.get(self.0.len() - 2).map(|s| s.as_str())
    }

    pub fn is_module_path(&self) -> bool {
        self.0.len() > 1
    }

    pub fn module_path(&self) -> Vec<&str> {
        self.0[..self.0.len() - 1]
            .iter()
            .map(|s| s.as_str())
            .collect()
    }

    pub fn vec(&self) -> Vec<&str> {
        self.0.iter().map(|s| s.as_str()).collect()
    }
}
