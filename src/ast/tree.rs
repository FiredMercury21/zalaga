use super::ast_types::*;
use super::ast_types::{ExprKind::*, NodeKind::*, TokType::*};
use crate::diagnostics::PathCache;
use ParseErrorKind::*;
use std::collections::HashMap;

/*---Types---*/

// What we pass to every function.
// I wanted to use an iterator but there's a
// couple times we need to go back.

#[derive(Debug, PartialEq)]
pub struct Cursor<'a> {
    pub stream: Vec<Token>,
    pub pos: usize,
    pub node_id: Id,
    pub cache: &'a mut PathCache,
}

impl<'a> Iterator for Cursor<'a> {
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

impl<'a> Cursor<'a> {
    pub fn peek(&self) -> Option<TokType> {
        self.stream
            .get(self.pos)
            .map(|Token { tok_type, .. }| tok_type.clone())
    }

    pub fn last_idx(&self) -> Span {
        // Empty streams should be handled before Cursor is created.
        // Usually we use this function after we read a bad token.
        let last_idx = self.pos.min(self.stream.len()).saturating_sub(1);
        self.stream[last_idx].span()
    }

    pub fn new_id(&mut self) -> Id {
        self.node_id.0 += 1;
        self.node_id
    }

    /// Expect a given token, else err generic.
    pub fn expect(&mut self, expected: TokType) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            t => Err(ParseError {
                err: ParseErrorKind::InvalidSyntax { found: t },
                span: self.last_idx(),
            }),
        }
    }

    /// Expect a given token, else err with given error.
    pub fn expect_else(
        &mut self,
        expected: TokType,
        error: impl FnOnce(Option<TokType>) -> ParseErrorKind,
    ) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            t => Err(ParseError {
                err: error(t),
                span: self.last_idx(),
            }),
        }
    }

    /// Expect an Ident token, return it as a String, else err generic.
    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            t => Err(ParseError {
                err: ParseErrorKind::InvalidSyntax { found: t },
                span: self.last_idx(),
            }),
        }
    }

    /// Expect an Ident token, return it as a String, else err with given error.
    pub fn expect_ident_else(
        &mut self,
        error: impl FnOnce(Option<TokType>) -> ParseErrorKind,
    ) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            t => Err(ParseError {
                err: error(t),
                span: self.last_idx(),
            }),
        }
    }

    pub fn new_node(&mut self, from: NodeKind) -> Node {
        Node {
            node: from,
            span: self.last_idx(),
            id: self.new_id(),
        }
    }

    pub fn new_expr(&mut self, from: ExprKind) -> Expr {
        Expr {
            expr: from,
            span: self.last_idx(),
            id: self.new_id(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub err: ParseErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    BadExpr { found: Option<TokType> },
    BadPath { found: Option<TokType> },
    BadNum { found: String },
    BadFloat { found: String },
    BadPattern { found: Option<TokType> },
    BadNegation { found: Option<TokType> },
    FnNoRetType,
    FnNoParen,
    FnNoName,
    FnNoBody,
    FnBadArg { found: Option<TokType> },
    FnSyntax { found: Option<TokType> },
    FnNoCloseBrack,
    VarNoType,
    VarNoName,
    VarNoAnnotation,
    ForNoInit,
    ForNoPred,
    ForNoBlock,
    WhileNoBlock,
    AsnBadSyntax { found: Option<TokType> },
    EnumNoBlock,
    EnumBadSyntax { found: Option<TokType> },
    EnumDuplicateVariant { found: String },
    StructNoBlock,
    StructBadSyntax { found: Option<TokType> },
    StructNoFieldInit,
    StructDuplicateField { found: String },
    BadType { found: Option<TokType> },
    IfNoBlock,
    BlockParseErr { found: Option<TokType> },
    ExprParseErr { found: Option<TokType> },
    UnclosedBrack,
    InvalidKeyword { found: Option<TokType> },
    InvalidField { found: Option<TokType> },
    ModuleNotFound { found: Path },
    ImportNoName,
    ImportNoAlias,
    ErrInModule { path: Path, err: Box<ParseError> },
    InvalidSyntax { found: Option<TokType> },
    UnexpectedEof,
    EmptyFile,
    Generic,
}

/*---Helper functions---*/

// String to binary operator
fn is_bin_op(op: &Operator) -> bool {
    use Operator::*;
    matches!(
        op,
        Add | Sub
            | Mul
            | Div
            | Exp
            | Mod
            | LT
            | GT
            | ET
            | LorET
            | GorET
            | NotET
            | Or
            | And
            | Assign
    )
}

// String to unary operator
fn is_un_op(op: &Operator) -> bool {
    use Operator::*;
    matches!(op, Not | Inc | Dec | Ref | Deref)
}

// Operator to precedence.
// Higher value is higher precedence.
// Remember that unary ops are all precedence 25.
fn op_to_prec(op: &Operator) -> Option<i32> {
    use Operator::*;

    Some(match op {
        Add => 10,
        Sub => 10,
        Mul => 15,
        Div => 15,
        Exp => 20,
        Mod => 15,
        LT => 7,
        GT => 7,
        ET => 6,
        LorET => 7,
        GorET => 7,
        NotET => 6,
        Or => 4,
        And => 5,
        Assign => 3,

        _ => return None,
    })
}

// Find appropriate parse function.
fn match_to_parse(code: &mut Cursor) -> Result<Node, ParseError> {
    Ok(match code.peek() {
        Some(Ident(ident)) => match ident.as_str() {
            "fn" => parse_fn_dec(code)?,
            "var" => parse_var_dec(code)?,
            "enum" => parse_enum_dec(code)?,
            "struct" => parse_struct_dec(code)?,
            "use" => parse_use(code)?,
            "for" => parse_for(code)?,
            "while" => parse_while(code)?,

            _ => {
                let expr = parse_expr(code, 0)?;
                code.new_node(Statement { expr })
            }
        },

        // If the thing has a unary op or is in brackets, it's an expression.
        Some(Op(_) | LBrack) => {
            let expr = parse_expr(code, 0)?;
            code.new_node(Statement { expr })
        }

        // I believe this is already handled by parse_expr. We can probably use parse_expr.
        Some(Indent | LSquirl) => {
            let expr = parse_block(code)?;
            code.new_node(Statement { expr })
        }

        t => {
            return Err(ParseError {
                err: InvalidKeyword { found: t },
                span: code.last_idx(),
            });
        }
    })
}

/*---Parsers---*/

pub fn parse_file(
    code: Vec<Token>,
    source: &str,
    cache: &mut PathCache,
) -> Result<Node, ParseError> {
    if code.is_empty() {
        return Err(ParseError {
            err: ParseErrorKind::EmptyFile,
            span: Span {
                source: Path::from(source),
                start: 0,
                end: 0,
            },
        });
    }
    let mut cursor = Cursor {
        stream: code,
        pos: 0,
        node_id: Id(0),
        cache,
    };

    let mut global = Vec::new();

    while let Some(token) = cursor.peek() {
        match token {
            Eof => {
                cursor.next();
                break;
            }
            Newline => {
                cursor.next();
                continue;
            }
            _ => global.push(match_to_parse(&mut cursor)?),
        }
    }

    Ok(cursor.new_node(Module {
        name: source.to_owned(),
        global,
    }))
}

// Blocks are whitespace-significant.
fn parse_block(code: &mut Cursor) -> Result<Expr, ParseError> {
    let mut statements = Vec::new();

    if let Some(Indent) = code.peek() {
        code.next();
    }

    // Check for terminators, otherwise push nodified line to statements.
    while let Some(token) = code.peek() {
        match token {
            Dedent | RSquirl => {
                code.next();
                break;
            }
            Newline => {
                code.next();
                continue;
            }
            Eof => break,

            _ => statements.push(match_to_parse(code)?),
        }
    }

    Ok(code.new_expr(ExprKind::Block { lines: statements }))
}

fn parse_fn_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // fn name(arg1: type, arg2: type) -> ret_type {  }

    code.expect_ident()?; // Should never err.
    let name = code.expect_ident_else(|_| FnNoName)?;
    code.expect_else(LBrack, |_| FnNoParen)?;

    let mut args = Vec::new();
    loop {
        if Some(RBrack) == code.peek() {
            break;
        }
        let arg = code.expect_ident()?;
        code.expect_else(Colon, |_| VarNoType)?;
        let var_type = Box::new(parse_type(code)?);

        args.push(code.new_node(VarDec {
            name: arg,
            expr: None,
            var_type,
        }));
        if Some(Comma) == code.peek() {
            code.next();
        } else {
            break;
        }
    }

    code.expect_else(RBrack, |_| FnNoParen)?;
    code.expect_else(Arrow, |_| FnNoRetType)?;

    let Ok(ret_type) = parse_type(code) else {
        return Err(ParseError {
            err: FnNoRetType,
            span: code.last_idx(),
        });
    };
    let ret_type = Box::new(ret_type);

    code.expect_else(Colon, |_| FnNoRetType)?;
    code.expect_else(Newline, |t| FnSyntax { found: t })?;

    let body = parse_block(code)?;

    Ok(code.new_node(FnDec {
        name,
        args,
        ret_type,
        body,
    }))
}

fn parse_var_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // var name: type
    // var name: type = stuff

    code.expect_ident()?;
    let name = code.expect_ident_else(|_| VarNoName)?;

    let infer_type: bool;
    let var_type = if matches!(code.peek(), Some(Colon)) {
        infer_type = false;
        code.expect(Colon)?;
        Box::new(parse_type(code)?)
    } else {
        infer_type = true;
        Box::new(code.new_node(Type {
            name: TypeNode::Infer,
        }))
    };
    let expr = if let Some(Op(Operator::Assign)) = code.peek() {
        code.next();
        Some(parse_expr(code, 0)?)
    } else {
        // Can't infer type if no expr.
        if infer_type {
            return Err(ParseError {
                err: VarNoAnnotation,
                span: code.last_idx(),
            });
        }

        None
    };

    Ok(code.new_node(VarDec {
        name,
        expr,
        var_type,
    }))
}

fn parse_fn_args(code: &mut Cursor) -> Result<Vec<Expr>, ParseError> {
    // (arg1, arg2, arg3)

    code.expect(LBrack)?;

    let mut args = Vec::new();
    while !matches!(code.peek(), Some(RBrack)) {
        args.push(parse_expr(code, 0)?);
        match code.peek() {
            // TODO: refactor.
            Some(RBrack) => break,
            Some(Comma) => {
                code.next();
            }
            t => {
                return Err(ParseError {
                    err: FnBadArg { found: t },
                    span: code.last_idx(),
                });
            }
        }
    }
    code.expect_else(RBrack, |_| FnNoCloseBrack)?;

    Ok(args)
}

// TODO: Treat a Sub at the start as a negative sign. var a: int = -3.
fn parse_expr(code: &mut Cursor, prec: i32) -> Result<Expr, ParseError> {
    // func( (a / 2), 3);
    // 1 + 1;
    // ( func(arg1, arg2) + x ) * y;
    // mystruct[ field1 = func(x); field2 = 2 + 3 ].field2 + 5 == 10

    // This is a tough one. Expressions can be recursive.

    let token = match code.next() {
        Some(token) => token,
        None => {
            return Err(ParseError {
                err: UnexpectedEof,
                span: code.last_idx(),
            });
        }
    };
    let mut current = match token {
        // Constant numbers.
        Num(num) => code.new_expr(Const {
            val: Constant::Num(num.parse().unwrap()),
        }),

        Float(num) => code.new_expr(Const {
            val: Constant::Float(num.parse().unwrap()),
        }),

        Char(c) => code.new_expr(Const {
            val: Constant::Char(c),
        }),

        // Bracketed expressions.
        LBrack => {
            let expr = parse_expr(code, 0)?;
            code.expect_else(RBrack, |_| UnclosedBrack)?;
            expr
        }

        // Block expressions.
        LSquirl | Indent => parse_block(code)?,

        // Unary operators.
        // Both unary operators and Sub (negative) parse at precedence 25, higher than
        // most other operators, ensuring proper binding. I think.
        Op(op) if is_un_op(&op) => {
            let expr = Box::new(parse_expr(code, 25)?);
            code.new_expr(UnOp { op, expr })
        }

        // Negative numbers.
        // We change 'Sub' to 'Neg' here.
        Op(Operator::Sub) => {
            let expr = Box::new(parse_expr(code, 25)?);
            code.new_expr(UnOp {
                op: Operator::Neg,
                expr,
            })
        }

        // If statement
        Ident(key) if key == "if" => {
            code.pos -= 1; // Don't like this.
            parse_if(code)?
        }

        // Break
        Ident(key) if key == "break" => code.new_expr(Break),

        // Continue
        Ident(key) if key == "continue" => code.new_expr(Continue),

        // Return statement
        Ident(key) if key == "return" => {
            code.pos -= 1;
            parse_return(code)?
        }

        Ident(key) if key == "true" => code.new_expr(Const {
            val: Constant::Bool(true),
        }),

        Ident(key) if key == "false" => code.new_expr(Const {
            val: Constant::Bool(false),
        }),

        // Match statement
        Ident(key) if key == "match" => {
            code.pos -= 1;
            parse_match(code)?
        }

        // Enum literal.
        Ident(variant) if matches!(code.peek(), Some(At)) => {
            // variant@myenum[ val ]
            // empty@myenum

            code.expect(At)?;
            // TODO: Replace with something that respects module paths!
            let path = parse_path(code)?;
            let val = parse_enum(code)?;

            code.new_expr(Enum { path, variant, val })
        }

        Ident(_) => {
            code.pos -= 1;
            let path = parse_path(code)?;
            match code.peek() {
                // Function call.
                Some(LBrack) => {
                    let args = parse_fn_args(code)?;
                    code.new_expr(FnCall { path, args })
                }
                // Struct literal.
                Some(LSquare) => {
                    let fields = parse_struct(code)?;
                    code.new_expr(Struct { path, fields })
                }
                // Variable.
                _ => code.new_expr(Var { path }),
            }
        }

        t => {
            return Err(ParseError {
                err: BadExpr { found: Some(t) },
                span: code.last_idx(),
            });
        }
    };

    loop {
        // Field access. Duplicates parse_atom, because that doesn't work on structs. Refactor?
        if matches!(code.peek(), Some(Period)) {
            code.next();
            let field = code.expect_ident_else(|t| InvalidField { found: t })?;
            current = code.new_expr(Field {
                base: Box::new(current),
                field,
            });
            continue;
        }

        // Pratt Parser: Binary Operators!
        let Some(Op(op)) = code.peek() else { break };
        if !is_bin_op(&op) {
            break;
        }

        // Check if precedence tells us to leave.
        let op_prec = op_to_prec(&op).unwrap();
        if op_prec < prec {
            break;
        }
        code.next();

        // Associativity
        let new_prec = match op {
            Operator::Assign => op_prec, // Right-associative
            _ => op_prec + 1,
        };

        let second = Box::new(parse_expr(code, new_prec)?);

        current = code.new_expr(BinOp {
            first: Box::new(current),
            op,
            second,
        });
    }

    Ok(current)
}

fn parse_for(code: &mut Cursor) -> Result<Node, ParseError> {
    // for (
    code.expect_ident()?;
    code.expect(LBrack)?;

    // var i: int = 0; i < 12; ++i
    let init = Box::new(parse_var_dec(code)?);
    code.expect_else(SColon, |_| ForNoInit)?;
    let pred = parse_expr(code, 0)?;
    code.expect_else(SColon, |_| ForNoPred)?;
    let then = parse_expr(code, 0)?;

    // ):
    code.expect_else(RBrack, |_| UnclosedBrack)?;
    code.expect_else(Colon, |_| ForNoBlock)?;

    let block = parse_block(code)?;

    Ok(code.new_node(For {
        init,
        pred,
        then,
        block,
    }))
}

fn parse_while(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;

    let pred = parse_expr(code, 0)?;
    code.expect_else(Colon, |_| WhileNoBlock)?;
    let block = parse_block(code)?;

    Ok(code.new_node(While { pred, block }))
}

fn parse_match(code: &mut Cursor) -> Result<Expr, ParseError> {
    code.expect_ident()?;
    let expr = Box::new(parse_expr(code, 0)?);
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;

    let mut grds = Vec::new();
    loop {
        // Don't like this generic break thingy.
        if Some(Guard) != code.peek() {
            break;
        }
        code.next();
        let patt = parse_pattern(code)?;
        code.expect(Arrow)?;

        // I don't like this. Generic expression, but has its own scope.
        // Only blocks are supposed to have own scopes? Idk.
        let expr = parse_expr(code, 0)?;

        grds.push(code.new_node(NodeKind::Guard { patt, expr }));
        // Weird. If no trailing comma, break.
        if code.peek() == Some(Newline) {
            break;
        }
        code.expect(Comma)?;
        code.expect(Newline)?;
    }

    Ok(code.new_expr(Match { expr, grds }))
}

fn parse_pattern(code: &mut Cursor) -> Result<Pattern, ParseError> {
    // x
    // 3
    // Variant(x)
    // _

    // The four types of pattern! Val, Var, Variant, All.
    Ok(match code.next() {
        Some(Ident(var)) => match code.peek() {
            // Variant
            Some(LBrack) => {
                code.next();
                let payload = code.expect_ident_else(|t| BadPattern { found: t })?;
                code.expect_else(RBrack, |t| BadPattern { found: t })?;
                Pattern::Variant { name: var, payload }
            }

            // Variable
            _ => Pattern::Var { name: var },
        },

        // Values
        Some(Num(num)) => Pattern::Val {
            val: Constant::Num(num.parse().map_err(|_| ParseError {
                err: BadNum { found: num },
                span: code.last_idx(),
            })?),
        },
        Some(Float(num)) => Pattern::Val {
            val: Constant::Float(num.parse().map_err(|_| ParseError {
                err: BadFloat { found: num },
                span: code.last_idx(),
            })?),
        },
        Some(Char(c)) => Pattern::Val {
            val: Constant::Char(c),
        },

        // All
        Some(Underscore) => Pattern::All,

        t => {
            return Err(ParseError {
                err: BadPattern { found: t },
                span: code.last_idx(),
            });
        }
    })
}

fn parse_use(code: &mut Cursor) -> Result<Node, ParseError> {
    // use std
    // use std::vec
    // use std::vec@alias
    // use longfilename@alias

    use crate::cli::utils::ast_path_to_file;
    use crate::cli::utils::targets::*;

    code.expect_ident()?;
    let module_path = parse_path(code)?;
    let mod_ast = match ast_path_to_file(&module_path, code.cache) {
        Ok(file_str) => match build_ast(&file_str, &format!("{}", module_path), code.cache) {
            Ok(mod_ast) => mod_ast,

            Err(e) => {
                return Err(ParseError {
                    err: ParseErrorKind::ErrInModule {
                        path: module_path.clone(),
                        err: Box::new(e),
                    },
                    span: code.last_idx(),
                });
            }
        },
        Err(_) => {
            return Err(ParseError {
                err: ModuleNotFound { found: module_path },
                span: code.last_idx(),
            });
        }
    };
    let root = Box::new(mod_ast);
    let name = if matches!(code.peek(), Some(At)) {
        code.next();
        code.expect_ident_else(|_| ParseErrorKind::ImportNoAlias)?
    } else {
        module_path.base().to_string()
    };
    Ok(code.new_node(Use { name, root }))
}

fn parse_return(code: &mut Cursor) -> Result<Expr, ParseError> {
    code.expect_ident()?;

    let val = match code.peek() {
        // What other tokens mean no expr?
        Some(Newline) => None,
        _ => Some(Box::new(parse_expr(code, 0)?)),
    };

    Ok(code.new_expr(Return { val }))
}

fn parse_enum_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect_else(Colon, |_| EnumNoBlock)?;
    code.expect_else(Newline, |_| EnumNoBlock)?;
    code.expect_else(Indent, |_| EnumNoBlock)?;
    let mut variants = Vec::new();
    loop {
        let name = code.expect_ident()?;

        // Check for any duplicate variants.
        if variants.iter().any(|v: &EnumVariant| v.name == name) {
            return Err(ParseError {
                err: EnumDuplicateVariant { found: name },
                span: code.last_idx(),
            });
        }

        variants.push(EnumVariant {
            name,
            var_type: {
                match code.peek() {
                    Some(Colon) => {
                        code.next();
                        Some(Box::new(parse_type(code)?))
                    }
                    _ => None,
                }
            },
        });
        match code.next() {
            Some(Newline) => {
                code.expect(Dedent)?;
                break;
            }

            Some(Comma) => {
                code.expect(Newline)?;
                continue;
            }

            t => {
                return Err(ParseError {
                    err: EnumBadSyntax { found: t },
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(code.new_node(EnumDec { name, variants }))
}

fn parse_enum(code: &mut Cursor) -> Result<Option<Box<Expr>>, ParseError> {
    if code.peek() != Some(LSquare) {
        return Ok(None);
    }
    code.expect_else(LSquare, |t| EnumBadSyntax { found: t })?;
    let payload = Box::new(parse_expr(code, 0)?);
    code.expect_else(RSquare, |t| EnumBadSyntax { found: t })?;
    Ok(Some(payload))
}

fn parse_struct_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect_else(Colon, |_| StructNoBlock)?;
    code.expect_else(Newline, |_| StructNoBlock)?;
    code.expect_else(Indent, |_| StructNoBlock)?;

    let mut fields = Vec::new();
    loop {
        let field = code.expect_ident_else(|t| StructBadSyntax { found: t })?;

        // Check for any duplicate fields.
        // Maybe use HashSet instead of Vec?
        if fields.iter().any(|f: &Node| {
            let VarDec { name, .. } = f.node.clone() else {
                unreachable!()
            };
            name == field
        }) {
            return Err(ParseError {
                err: StructDuplicateField { found: field },
                span: code.last_idx(),
            });
        }

        code.expect(Colon)?;
        let var_type = Box::new(parse_type(code)?);
        fields.push(code.new_node(VarDec {
            name: field,
            expr: None,
            var_type,
        }));

        // Check for end of struct dec.
        match code.next() {
            Some(Newline) => {
                code.expect(Dedent)?;
                break;
            }

            Some(Comma) => {
                code.expect(Newline)?;
                continue;
            }

            t => {
                return Err(ParseError {
                    err: StructBadSyntax { found: t },
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(code.new_node(StructDec { name, fields }))
}

fn parse_struct(code: &mut Cursor) -> Result<HashMap<String, Expr>, ParseError> {
    code.expect(LSquare)?;
    let mut fields = HashMap::new();
    loop {
        let field = code.expect_ident_else(|t| StructBadSyntax { found: t })?;

        code.expect_else(Op(Operator::Assign), |_| StructNoFieldInit)?;

        let second = parse_expr(code, 0)?;

        fields.insert(field, second);

        match code.next() {
            Some(RSquare) => break,

            Some(Comma) => continue,

            t => {
                return Err(ParseError {
                    err: StructBadSyntax { found: t },
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(fields)
}

fn parse_path(code: &mut Cursor) -> Result<Path, ParseError> {
    let mut path = Path::default();
    loop {
        path.push(code.expect_ident_else(|t| BadPath { found: t })?);
        if matches!(code.peek(), Some(Separator)) {
            code.next();
        } else {
            break;
        }
    }
    Ok(path)
}

fn parse_if(code: &mut Cursor) -> Result<Expr, ParseError> {
    // if foo == bar:
    //     expression
    // elif baz:
    //     expression
    // else:

    code.expect_ident()?;

    let pred = Box::new(parse_expr(code, 0)?);

    code.expect_else(Colon, |_| IfNoBlock)?;
    code.expect_else(Newline, |_| IfNoBlock)?;

    let then = Box::new(parse_block(code)?);

    let else_block = if let Some(Ident(tok)) = code.peek() {
        match tok.as_str() {
            "else" => {
                code.next();
                code.expect_else(Colon, |_| IfNoBlock)?;
                code.expect_else(Newline, |_| IfNoBlock)?;
                Some(Box::new(parse_block(code)?))
            }
            "elif" => Some(Box::new(parse_if(code)?)),
            _ => None,
        }
    } else {
        None
    };

    Ok(code.new_expr(If {
        pred,
        then,
        else_block,
    }))
}

// Really weird function, weird syntax, simple logic.
fn parse_type(code: &mut Cursor) -> Result<Node, ParseError> {
    use TypeNode::*;

    Ok(Node {
        node: Type {
            name: {
                let mut ref_n = 0;
                let mut base = loop {
                    // Find base type, track reference number.
                    match code.peek() {
                        // Base type.
                        Some(Ident(_)) => {
                            break Base(parse_path(code)?);
                        }
                        // Track number of references.
                        Some(Op(Operator::Ref)) => {
                            code.next();
                            ref_n += 1
                        }
                        // Ewww. To handle '&&' turning into 'And' in lexer.
                        Some(Op(Operator::And)) => {
                            code.next();
                            ref_n += 2
                        }
                        t => {
                            return Err(ParseError {
                                err: BadType { found: t },
                                span: code.last_idx(),
                            });
                        }
                    }
                };
                // Wrap the type in counted references.
                for _ in 0..ref_n {
                    base = Ref(Box::new(base));
                }
                base
            },
        },
        span: code.last_idx(),
        id: code.new_id(),
    })
}

/*---Tests---*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::lexer::tokenize_code;
    use crate::cli::utils::ast_path_to_file;

    #[test]
    fn test_var_asn() {
        let test = "var my_var: int = 0\nvar vartwo: &stuff";
        let thing = tokenize_code(test, &std::path::PathBuf::from("test.zg"));
        let mut cache = PathCache::new();
        assert!(parse_file(thing, &"var_asn".to_string(), &mut cache).is_ok());
    }

    #[test]
    fn test_quicksort_ast() {
        let mut cache = PathCache::new();
        let contents = ast_path_to_file(
            &Path(vec!["examples".to_string(), "quicksort".to_string()]),
            &mut cache,
        )
        .unwrap();
        assert!(
            parse_file(
                tokenize_code(&contents, &std::path::PathBuf::from("quicksort.zg")),
                "quicksort.zg",
                &mut cache
            )
            .is_ok()
        );
    }
}
