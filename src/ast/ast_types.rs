use super::lexer::TokType::*;
use super::lexer::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Id(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Num(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub node: NodeType,
    pub span: Span,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub err: ParseErrorType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub expr: ExprType,
    pub span: Span,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprType {
    Var {
        name: String,
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
        scope: Vec<Node>,
    },
    FnCall {
        name: String,
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
        name: String,
        fields: Vec<Expr>, // Each is a BinOp with Operator::Assign.
    },
    Enum {
        name: String,
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
    Break {
        val: Option<Box<Expr>>,
    },
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Module {
        name: String,
        scope: Vec<Node>, // TODO: Should rename this 'global'.
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
        pred: Expr,
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
        name: Box<Node>,
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
    Base(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorType {
    BadDeref,
    BadRef,
    BadAtom,
    BadExpr,
    FnNoRetType,
    FnNoParen,
    FnNoName,
    FnNoBody,
    FnBadArg,
    FnSyntax,
    FnNoCloseBrack,
    VarNoType,
    VarNoName,
    ForNoInit,
    ForNoPred,
    ForNoBlock,
    WhileNoBlock,
    AsnBadSyntax,
    EnumNoBlock,
    EnumBadSyntax,
    StructNoBlock,
    StructBadSyntax,
    StructNoFieldInit,
    BadType,
    UnionNoBlock,
    UnionBadSyntax,
    IfNoBlock,
    BlockParseErr,
    ExprParseErr,
    UnclosedBrack,
    InvalidKeyword,
    InvalidField,
    InvalidSyntax,
    UnexpectedEof,
    ScopeError,
    EmptyFile,
    Generic,
}

// What we pass to every function.
// I wanted to use an iterator but there's a
// couple times we need to go back.
#[derive(Debug, Clone, PartialEq)]
pub struct Cursor {
    pub stream: Vec<Token>,
    pub pos: usize,
    pub node_id: Id,
}

impl Iterator for Cursor {
    type Item = TokType;
    fn next(&mut self) -> Option<TokType> {
        let ret = self
            .stream
            .get(self.pos)
            .map(|Token { tok_type, .. }| tok_type.clone());
        self.pos += 1;
        ret
    }
}

impl Cursor {
    pub fn peek(&self) -> Option<TokType> {
        self.stream
            .get(self.pos)
            .map(|Token { tok_type, .. }| tok_type.clone())
    }

    pub fn last_idx(&self) -> Span {
        // Ideally there should be checks on empty streams.
        // Usually we use this function after we read a bad token.
        match self.stream.get(self.pos - 1) {
            Some(tok) => tok.index.clone(),
            None => self.stream[self.pos - 2].index.clone(),
        }
    }

    pub fn new_id(&mut self) -> Id {
        self.node_id.0 += 1;
        self.node_id
    }

    // Expect a certain token, else err.
    pub fn expect(&mut self, expected: TokType) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            _ => Err(ParseError {
                err: ParseErrorType::InvalidSyntax,
                span: self.last_idx(),
            }),
        }
    }

    // Expect a certain token, else err with given error.
    pub fn expect_else(
        &mut self,
        expected: TokType,
        error: ParseErrorType,
    ) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            _ => Err(ParseError {
                err: error,
                span: self.last_idx(),
            }),
        }
    }

    // Expect an ident, return it else err.
    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            _ => Err(ParseError {
                err: ParseErrorType::InvalidSyntax,
                span: self.last_idx(),
            }),
        }
    }

    // Expect an ident, return it else err with given error.
    pub fn expect_ident_else(&mut self, error: ParseErrorType) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            _ => Err(ParseError {
                err: error,
                span: self.last_idx(),
            }),
        }
    }

    pub fn new_node(&mut self, from: NodeType) -> Node {
        Node {
            node: from,
            span: self.last_idx(),
            id: self.new_id(),
        }
    }

    pub fn new_expr(&mut self, from: ExprType) -> Expr {
        Expr {
            expr: from,
            span: self.last_idx(),
            id: self.new_id(),
        }
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
