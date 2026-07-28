#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    All,
    Var { name: String },
    Val { val: Constant },
    Variant { name: String, payload: String },
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
        args: Vec<Expr>,
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
        fields: Vec<Expr>, // Each is a BinOp with Operator::Assign.
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path(pub Vec<String>);

impl Path {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn from_str(s: &str) -> Self {
        Self(vec![s.to_string()])
    }

    pub fn push(&mut self, name: String) {
        self.0.push(name);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn first(&self) -> String {
        self.0[0].clone()
    }

    pub fn pop_first(&mut self) {
        self.0.remove(0);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn base(&self) -> String {
        self.0[self.0.len() - 1].clone()
    }

    pub fn fname(&self) -> Option<String> {
        self.0.get(self.0.len() - 2).cloned()
    }

    pub fn is_module_path(&self) -> bool {
        self.0.len() > 1
    }

    pub fn module_path(&self) -> Vec<String> {
        self.0[..self.0.len() - 1].to_vec()
    }

    pub fn vec(&self) -> Vec<String> {
        self.0.clone()
    }
}

// Display.
/*
impl std::fmt::Display for Node {
    pub fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_indent(f, Vec::new())
    }
}

impl Expr {

}

impl Node {
    fn fmt_indent(&self, f: &mut std::fmt::Formatter<'_>, pipes: Vec<bool>) -> std::fmt::Result {
        use Node::*;

        let pre = if pipes.is_empty() {
            "".to_string()
        } else {
            pipes.iter().fold(String::new(), |acc, i| {
                acc + &match i {
                    3 => "└─",
                    2 => "├─",
                    1 => "│ ",
                    _ => "  ",
                }
            })
        };
        for i in 0..pipes.len() {
            if pipes[i] == 3 {
                pipes[i] = 0;
            }
        }
        write!(f, "{pre}")?;
        match self {
            Module { name, root } => {
                writeln!(f, "Module '{name}'\n{pre}Module root:")?;
                pipes.push(3);
                root.fmt_indent(f, pipes)
            }
            FnDec {
                name,
                args,
                ret_type,
                body,
            } => {
                writeln!(f, "FnDec '{name}'\n{pre}args:")?;
                if args.is_empty() {
                    writeln!(f, "{pre}├─No arguments.")?;
                } else {
                    writeln!(f, "{pre}├─args:")?;
                    pipes.push(2);
                    for i in 0..(args.len() - 1) {
                        args[i].fmt_indent(f, pipes)?;
                    }
                    pipes.pop();
                    pipes.push(3);
                    args[args.len() - 1].fmt_indent(f, pipes)?;
                }
                writeln!(f, "{pre}ret_type:")?;
                ret_type.fmt_indent(f, {
                    pipes.push(2);
                    pipes
                })?;
                writeln!(f, "{pre}└─body:")?;
                body.fmt_indent(f, {
                    pipes.push(3);
                    pipes
                })
            }
            Block { scope } => {
                writeln!(f, "Block, scope:")?;
                pipes.push(2);
                if scope.is_empty() {
                    writeln!(f, "{pre}└─No arguments.")
                } else {
                    writeln!(f, "{pre}└─args:")?;
                    pipes.push(2);
                    for i in 0..(args.len() - 1) {
                        args[i].fmt_indent(f, pipes)?;
                    }
                    pipes.pop();
                    pipes.push(3);
                    args[args.len() - 1].fmt_indent(f, pipes)
                }
                Ok(())
            }
            FnCall { name, args } => {
                writeln!(f, "FnCall '{name}'")?;
                if args.is_empty() {
                    writeln!(f, "{pre}└─No arguments.")
                } else {
                    writeln!(f, "{pre}└─args:")?;
                    pipes.push(2);
                    for i in 0..(args.len() - 1) {
                        args[i].fmt_indent(f, pipes)?;
                    }
                    pipes.pop();
                    pipes.push(3);
                    args[args.len() - 1].fmt_indent(f, pipes)
                }
            }
            Expr { expr } => {
                writeln!(f, "{pre}Expr:")?;
                expr.fmt_indent(f, indent + 1)
            }
            VarAsn { name, val } => {
                writeln!(f, "VarAsn '{name}'\n{pre}val:")?;
                val.fmt_indent(f, indent + 1)
            }
            VarDec {
                name,
                expr,
                var_type,
            } => {
                writeln!(f, "VarDec '{name}'\n{pre}type:")?;
                var_type.fmt_indent(f, indent + 1)?;
                match expr {
                    Some(expr) => {
                        writeln!(f, "{pre}val:")?;
                        expr.fmt_indent(f, indent + 1)
                    }
                    None => writeln!(f, "{pre}No initializer."),
                }
            }
            Var { name } => {
                writeln!(f, "Var '{name}'")
            }
            Ref { expr } => {
                writeln!(f, "Ref:")?;
                expr.fmt_indent(f, indent + 1)
            }
            Deref { expr } => {
                writeln!(f, "Deref:")?;
                expr.fmt_indent(f, indent + 1)
            }
            Field { base, field } => {
                writeln!(f, "Field Access, field '{field}' of:")?;
                base.fmt_indent(f, indent + 1)
            }
            StructDec { name, fields } => {
                writeln!(f, "StructDec '{name}'\n{pre}fields:")?;
                for node in fields {
                    node.fmt_indent(f, indent + 1)?;
                }
                Ok(())
            }
            UnionDec { name, variants } => {
                writeln!(f, "UnionDec '{name}'\n{pre}variants:")?;
                for node in variants {
                    node.fmt_indent(f, indent + 1)?;
                }
                Ok(())
            }
            EnumDec { name, variants } => {
                writeln!(f, "EnumDec '{name}'\n{pre}variants:")?;
                for node in variants {
                    writeln!(f, "{pre}| {node}")?;
                }
                Ok(())
            }
            Struct { name, fields } => {
                writeln!(f, "Struct '{name}'\n{pre}fields:")?;
                for node in fields {
                    node.fmt_indent(f, indent + 1)?;
                }
                Ok(())
            }
            Union { name, variant, val } => {
                writeln!(f, "Union '{name}', variant '{variant}'\n{pre}value:")?;
                val.fmt_indent(f, indent + 1)?;
                Ok(())
            }
            Enum { variant } => {
                writeln!(f, "Enum '{variant}'")?;
                Ok(())
            }
            For {
                init,
                pred,
                then,
                block,
            } => {
                writeln!(f, "For {init:?}\n{pre}pred:")?;
                pred.fmt_indent(f, indent + 1)?;
                writeln!(f, "{pre}then:")?;
                then.fmt_indent(f, indent + 1)?;
                writeln!(f, "{pre}block:")?;
                block.fmt_indent(f, indent + 1)
            }
            While { pred, block } => {
                writeln!(f, "While {pred:?}\n{pre}block:")?;
                block.fmt_indent(f, indent + 1)
            }
            If {
                pred,
                then,
                else_block,
            } => {
                writeln!(f, "If, pred:")?;
                pred.fmt_indent(f, indent + 1)?;
                writeln!(f, "{pre}then:")?;
                then.fmt_indent(f, indent + 1)?;
                match else_block {
                    Some(block) => {
                        writeln!(f, "{pre}else:")?;
                        block.fmt_indent(f, indent + 1)
                    }
                    None => Ok(()),
                }
            }
            BinOp { first, op, second } => {
                writeln!(f, "Operator {op:?}\n{pre}first:")?;
                first.fmt_indent(f, indent + 1)?;
                writeln!(f, "{pre}second:")?;
                second.fmt_indent(f, indent + 1)
            }
            UnOp { val, op } => {
                writeln!(f, "Operator {op:?}\n{pre}val:")?;
                val.fmt_indent(f, indent + 1)
            }
            Return { val } => {
                writeln!(f, "Return\n{pre}val:")?;
                val.fmt_indent(f, indent + 1)
            }
            Const { val } => {
                writeln!(f, "Const\n{pre}val: {val:?}")
            }

            _ => writeln!(f, "{self:?}"),
        }
    }
}
*/
