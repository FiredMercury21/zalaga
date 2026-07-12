use crate::ast::tree::ParseError::InvalidSyntax;

use super::lexer::TokType::*;
use super::lexer::*;

/*---Type Declarations---*/

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Num(i64),
    Float(f64),
}

/*
// TODO: Make Nodes have spans.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub node: NodeType,
    pub span: Span,
}

// TODO: Use a single expr type for Rust-style expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var {
        name: String,
    },
    VarAsn {
        name: String,
        val: Box<Node>,
    },
    Match {
        expr: Box<Node>,
        grds: Vec<Node>,
    },
    If {
        cond: Box<Node>,
        then: Box<Node>,
        else: Option<Box<Node>>,
    },
    Statement {
        expr: Box<Node>,
    },
    Block {
        scope: Vec<Node>,
    },
    FnCall {
        name: String,
        args: Vec<Node>,
    },
    Const {
        value: Constant,
    },
}
*/

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Module {
        name: String,
        root: Box<Node>,
    },
    FnDec {
        name: String,
        args: Vec<Node>,
        ret_type: Box<Node>,
        body: Box<Node>,
    },
    Block {
        scope: Vec<Node>,
    },
    FnCall {
        name: String,
        args: Vec<Node>,
    },
    Expr {
        expr: Box<Node>,
    },
    VarAsn {
        name: String,
        val: Box<Node>,
    },
    VarDec {
        name: String,
        expr: Option<Box<Node>>,
        var_type: Box<Node>,
    },
    Var {
        name: String,
    },
    Ref {
        expr: Box<Node>,
    },
    Deref {
        expr: Box<Node>,
    },
    Match {
        grds: Vec<Node>,
    },
    Guard {
        pred: Box<Node>,
        expr: Box<Node>,
    },
    Field {
        base: Box<Node>,
        field: String,
    },
    StructDec {
        name: String,
        fields: Vec<Node>,
    },
    Struct {
        name: String,
        fields: Vec<Node>,
    },
    UnionDec {
        name: String,
        variants: Vec<Node>,
    },
    EnumDec {
        name: String,
        variants: Vec<String>,
    },
    Union {
        variant: String,
    },
    Enum {
        variant: String,
    },
    For {
        init: Box<Node>,
        pred: Box<Node>,
        then: Box<Node>,
        block: Box<Node>,
    },
    While {
        pred: Box<Node>,
        block: Box<Node>,
    },
    If {
        pred: Box<Node>,
        block: Box<Node>,
        else_block: Option<Box<Node>>,
    },
    BinOp {
        first: Box<Node>,
        op: Operator,
        second: Box<Node>,
    },
    UnOp {
        val: Box<Node>,
        op: Operator,
    },
    Return {
        val: Box<Node>,
    },
    Const {
        val: Constant,
    },
    Use {
        name: Box<Node>,
    },
    Type {
        name: TypeType,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeType {
    Ref(Box<TypeType>),
    Base(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    FnNoRetType(Span),
    FnNoParen(Span),
    FnNoName(Span),
    FnNoBody(Span),
    FnBadArg(Span),
    VarNoType(Span),
    VarNoName(Span),
    BlockParseErr(Span),
    ExprParseErr(Span),
    InvalidSyntax(Span),
    EmptyFile,
    Generic,
}

// What we pass to every function.
// I wanted to use an iterator but there's a
// couple times we need to go back.
#[derive(Debug, Clone, PartialEq)]
struct Cursor {
    stream: Vec<Token>,
    pos: usize,
}

impl Iterator for Cursor {
    type Item = TokType;
    fn next(&mut self) -> Option<TokType> {
        let ret = match self.stream.get(self.pos) {
            Some(Token { tok_type, .. }) => Some(tok_type.clone()),
            None => None,
        };
        self.pos += 1;
        ret
    }
}

impl Cursor {
    fn peek(&self) -> Option<TokType> {
        match self.stream.get(self.pos) {
            Some(Token { tok_type, .. }) => Some(tok_type.clone()),
            None => None,
        }
    }

    fn last_idx(&self) -> Span {
        // Ideally there should be checks on empty streams.
        // Usually we use this function after we read a bad token.
        match self.stream.get(self.pos - 1) {
            Some(tok) => tok.index.clone(),
            None => self.stream[self.pos - 2].index.clone(),
        }
    }

    // Expect a certain token, else err.
    fn expect(&mut self, expected: TokType) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            _ => Err(InvalidSyntax(self.last_idx())),
        }
    }

    // Expect a certain token, else err with given error.
    fn expect_else(&mut self, expected: TokType, error: ParseError) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if token == expected => Ok(()),
            _ => Err(error),
        }
    }

    // Expect an ident, return it else err.
    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            _ => Err(InvalidSyntax(self.last_idx())),
        }
    }

    // Expect an ident, return it else err with given error.
    fn expect_ident_else(&mut self, error: ParseError) -> Result<String, ParseError> {
        match self.next() {
            Some(Ident(ident)) => Ok(ident),
            _ => Err(error),
        }
    }
}

/*---Helper functions---*/

// String to binary operator
fn is_bin_op(op: &Operator) -> bool {
    use Operator::*;
    matches!(
        op,
        Add | Sub | Mul | Div | Exp | Mod | LT | GT | ET | LorET | GorET | NotET | Or | And
    )
}

// String to unary operator
fn is_un_op(op: &Operator) -> bool {
    use Operator::*;
    matches!(op, Neg | Inc | Dec)
}

// Operator to precedence.
// Higher value is higher precedence.
fn op_to_prec(op: &Operator) -> Option<i32> {
    use Operator::*;

    Some(match op {
        Add => 10,
        Sub => 10,
        Mul => 15,
        Div => 15,
        Exp => 20,
        Mod => 15,
        LT => 5,
        GT => 5,
        ET => 5,
        LorET => 5,
        GorET => 5,
        NotET => 5,
        Or => 5,
        And => 5,

        _ => return None,
    })
}

// Don't think we use this... could be useful.
/*
fn is_end_key(c: &TokType) -> bool {
    use crate::ast::lexer::TokType::*;
    matches!(c, Eof | Guard | Comma | RBrack | RSquirl | SColon | Arrow | Indent | Dedent | Newline)
}
*/

// Find appropriate parse function.
fn match_to_parse(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;
    match code.peek() {
        Some(Ident(ident)) => {
            Ok(match ident.as_str() {
                "fn" => parse_fn_dec(code)?,
                "var" => parse_var_dec(code)?,
                "enum" => parse_enum_dec(code)?,
                "uni" => parse_union_dec(code)?,
                "struct" => parse_struct_dec(code)?,
                //"use"  => parse_use(code)?,
                "for" => parse_for(code)?,
                "if" => parse_if(code)?,
                "while" => parse_while(code)?,
                "return" => parse_return(code)?,
                "break" => Node::Break,
                "continue" => Node::Continue,

                // If token after ident is =
                _ if matches!(
                    code.stream.get(code.pos + 1),
                    Some(Token {
                        tok_type: Assign,
                        ..
                    })
                ) =>
                {
                    parse_var_asn(code)?
                }

                _ => parse_expr(code, 0)?,
            })
        }

        // If the thing is a pointer or in brackets, it's an expression.
        Some(Op(..)) | Some(LBrack) => Ok(parse_expr(code, 0)?),

        Some(LSquirl) => Ok(parse_block(code)?),

        _ => return Err(InvalidSyntax(code.last_idx())),
    }
}

/*---Parsers---*/

pub fn parse_file(code: Vec<Token>, name: &String) -> Result<Node, ParseError> {
    let mut cursor = Cursor {
        stream: code,
        pos: 0,
    };
    let root = Box::new(parse_block(&mut cursor)?);

    Ok(Node::Module {
        name: name.clone(),
        root: root,
    })
}

// Blocks are whitespace-significant.
fn parse_block(code: &mut Cursor) -> Result<Node, ParseError> {
    let mut statements = Vec::new();

    if let Some(Indent) = code.peek() {
        code.next();
    }

    while let Some(token) = code.peek() {
        match token {
            Dedent | Eof => {
                code.next();
                break;
            }
            Newline => {
                code.next();
                continue;
            }
            _ => statements.push(match_to_parse(code)?),
        }
    }

    Ok(Node::Block { scope: statements })
}

fn parse_fn_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // fn name(arg1: type, arg2: type) -> ret_type {  }

    use ParseError::*;

    code.expect_ident()?; // Should never err.
    let name = code.expect_ident_else(FnNoName(code.last_idx()))?;
    code.expect_else(LBrack, FnNoParen(code.last_idx()))?;

    let mut args = Vec::new();
    loop {
        if Some(RBrack) == code.peek() {
            break;
        }
        let arg = code.expect_ident()?;
        code.expect_else(Colon, VarNoType(code.last_idx()))?;
        let var_type = Box::new(parse_type(code)?);
        if Some(Comma) == code.peek() {
            code.next();
        }
        args.push(Node::VarDec {
            name: arg,
            expr: None,
            var_type,
        });
    }

    code.expect_else(RBrack, FnNoParen(code.last_idx()))?;
    code.expect_else(Arrow, FnNoRetType(code.last_idx()))?;
    let Ok(ret_type) = parse_type(code) else {
        return Err(FnNoRetType(code.last_idx()));
    };
    let ret_type = Box::new(ret_type);
    code.expect_else(Colon, FnNoParen(code.last_idx()))?;
    code.expect_else(Newline, FnNoParen(code.last_idx()))?;
    code.expect_else(Indent, FnNoParen(code.last_idx()))?;

    let fn_body = parse_block(code)?;
    let body = Box::new(fn_body);

    Ok(Node::FnDec {
        name,
        args,
        ret_type,
        body,
    })
}

fn parse_var_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // var name: type
    // var name: type = stuff

    use ParseError::*;

    code.expect_ident()?;
    let name = code.expect_ident_else(VarNoName(code.last_idx()))?;
    code.expect_else(Colon, VarNoType(code.last_idx()))?;
    let var_type = Box::new(parse_type(code)?);

    let expr = if let Some(Assign) = code.peek() {
        code.next();
        let expr = Some(Box::new(parse_expr(code, 0)?));
        expr
    } else {
        None
    };

    Ok(Node::VarDec {
        name,
        expr,
        var_type,
    })
}

fn parse_fn_args(code: &mut Cursor) -> Result<Vec<Node>, ParseError> {
    // (arg1, arg2, arg3)

    use ParseError::*;

    code.expect(LBrack)?;

    let mut args = Vec::new();
    while let Some(Ident(arg)) = code.next() {
        args.push(Node::Var { name: arg });
        let check = code.next();
        let Some(Comma) = check else {
            let Some(RBrack) = check else {
                return Err(FnBadArg(code.last_idx()));
            };
            break;
        };
    }

    Ok(args)
}

fn parse_expr(code: &mut Cursor, prec: i32) -> Result<Node, ParseError> {
    // func( (a / 2), 3);
    // 1 + 1;
    // ( func(arg1, arg2) + x ) * y;
    // mystruct[ field1 = func(x); field2 = 2 + 3 ].field2 + 5 == 10

    // This is a tough one. Expressions can be recursive.

    use Node::*;
    use ParseError::*;

    let Some(token) = code.next() else {
        return Err(ExprParseErr(code.last_idx()));
    };
    let mut current = match token {
        // Constant numbers.
        Num(num) => Const {
            val: Constant::Num(num.parse().unwrap()),
        },

        // Bracketed expressions.
        LBrack => {
            let expr = parse_expr(code, 0)?;
            code.expect(RBrack)?;
            expr
        }

        // Block expressions.
        LSquirl => parse_block(code)?,

        // Unary operators.
        // ERROR: Doesn't work if unary works on non-atoms. Like *(&arr + 1).
        // Is deref a unary op?
        Op(op) if is_un_op(&op) => {
            let val = Box::new(parse_atom(code)?);
            UnOp { val, op }
        }

        // Function calls.
        Ident(name) if matches!(code.peek(), Some(LBrack)) => {
            let args = parse_fn_args(code)?;
            FnCall { name, args }
        }

        // Struct def.
        Ident(name) if matches!(code.peek(), Some(LSquare)) => {
            let fields = parse_struct(code)?;
            Struct { name, fields }
        }

        // Idents?
        _ => {
            code.pos -= 1;
            parse_atom(code)?
        }
    };

    loop {
        // Field access. Duplicates parse_atom, because that doesn't work on structs. Refactor?
        while matches!(code.peek(), Some(Period)) {
            code.next();
            let field = code.expect_ident_else(InvalidSyntax(code.last_idx()))?;
            current = Field {
                base: Box::new(current),
                field,
            };
        }

        // Consumes last token. Should we?
        let Some(Op(op)) = code.peek() else { break };
        code.next();
        if !is_bin_op(&op) {
            break;
        }

        let new_prec = op_to_prec(&op).unwrap();
        if new_prec < prec {
            break;
        }
        let second = Box::new(parse_expr(code, new_prec + 1)?);

        current = BinOp {
            first: Box::new(current),
            op: op,
            second: second,
        };
    }

    Ok(current)
}

fn parse_atom(code: &mut Cursor) -> Result<Node, ParseError> {
    // Ref and deref have Ident after them unless bracks,
    // In which case expression.
    // Field access can follow
    use Node::*;
    use ParseError::*;

    let mut current = if let Some(token) = code.next() {
        match token {
            Op(Operator::Deref) => Deref {
                expr: Box::new(match code.peek() {
                    Some(LBrack) => {
                        let expr = parse_expr(code, 0)?;
                        code.expect(RBrack)?;
                        expr
                    }
                    Some(Ident(name)) => Var { name },
                    _ => return Err(InvalidSyntax(code.last_idx())),
                }),
            },

            Op(Operator::Ref) => Ref {
                expr: Box::new(match code.peek() {
                    Some(LBrack) => {
                        let expr = parse_expr(code, 0)?;
                        code.expect(RBrack)?;
                        expr
                    }
                    Some(Ident(name)) => Var { name },
                    _ => return Err(InvalidSyntax(code.last_idx())),
                }),
            },

            Ident(name) => Var { name },

            LBrack => {
                let expr = Box::new(parse_expr(code, 0)?);
                code.expect(RBrack)?;
                Expr { expr }
            }

            _ => return Err(InvalidSyntax(code.last_idx())),
        }
    } else {
        return Err(InvalidSyntax(code.last_idx()));
    };

    while matches!(code.peek(), Some(Period)) {
        code.next();
        let field = code.expect_ident_else(InvalidSyntax(code.last_idx()))?;
        current = Field {
            base: Box::new(current),
            field,
        };
    }

    Ok(current)
}

fn parse_for(code: &mut Cursor) -> Result<Node, ParseError> {
    // for (
    code.expect_ident()?;
    code.expect(LBrack)?;

    // var i: int = 0; i < 12; ++i
    let init = Box::new(parse_var_dec(code)?);
    code.expect(SColon)?;
    let pred = Box::new(parse_expr(code, 0)?);
    code.expect(SColon)?;
    let then = Box::new(parse_expr(code, 0)?);

    // ):
    code.expect(RBrack)?;
    code.expect(Colon)?;

    let block = Box::new(parse_block(code)?);

    Ok(Node::For {
        init: init,
        pred: pred,
        then: then,
        block: block,
    })
}

fn parse_while(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;

    let pred = Box::new(parse_expr(code, 0)?);
    code.expect(Colon)?;
    let block = Box::new(parse_block(code)?);

    Ok(Node::While {
        pred: pred,
        block: block,
    })
}

/*
 * This doesn't actually work right... Didn't add comparison.
fn parse_match(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;
    parse_expr(code, 0)?;
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;

    let mut guards = Vec::new();
    loop {
        if Some(Guard) != code.peek() {
            break;
        }
        code.next();
        let pred = Box::new(parse_expr(code, 0)?);
        code.expect(Arrow)?;
        let expr: Box<Node> = Box::new(parse_expr(code, 0)?);
        guards.push(Node::Guard {
            pred: pred,
            expr: expr,
        });
        // Comma? Does expr consume last token?
        code.expect(Newline)?;
    }

    Ok(Node::Match { grds: guards })
}
*/

fn parse_var_asn(code: &mut Cursor) -> Result<Node, ParseError> {
    let name = code.expect_ident()?;
    code.expect(Assign)?;

    let val = Box::new(parse_expr(code, 0)?);
    code.expect(Newline)?;

    Ok(Node::VarAsn {
        name: name,
        val: val,
    })
}

fn parse_return(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;

    let val = Box::new(parse_expr(code, 0)?);

    Ok(Node::Return { val: val })
}

fn parse_enum_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;
    let mut variants = Vec::new();
    while let Some(Ident(field)) = code.next() {
        variants.push(field);
        match code.next() {
            Some(Newline) => {
                code.expect(Dedent)?;
                break;
            }

            Some(Comma) => {
                code.expect(Newline)?;
                continue;
            }

            _ => return Err(InvalidSyntax(code.last_idx())),
        }
    }

    Ok(Node::EnumDec { name, variants })
}

fn parse_struct_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;

    let mut fields = Vec::new();
    while let Some(Ident(field)) = code.next() {
        code.expect(Colon)?;
        let var_type = Box::new(parse_type(code)?);
        fields.push(Node::VarDec {
            name: field,
            expr: None,
            var_type,
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

            _ => return Err(InvalidSyntax(code.last_idx())),
        }
    }

    Ok(Node::StructDec { name, fields })
}

fn parse_struct(code: &mut Cursor) -> Result<Vec<Node>, ParseError> {
    code.expect(LSquare)?;
    let mut fields = Vec::new();
    while let Some(Ident(field)) = code.next() {
        code.expect(Assign)?;
        let val = Box::new(parse_expr(code, 0)?);
        fields.push(Node::VarAsn { name: field, val });
        match code.next() {
            Some(RSquare) => {
                code.expect(Newline)?;
                break;
            }

            Some(Comma) => continue,

            _ => return Err(InvalidSyntax(code.last_idx())),
        }
    }

    Ok(fields)
}

fn parse_union_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;

    let mut variants = Vec::new();
    while let Some(Ident(field)) = code.next() {
        code.expect(Colon)?;
        let var_type = Box::new(parse_type(code)?);
        variants.push(Node::VarDec {
            name: field,
            expr: None,
            var_type,
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

            _ => return Err(InvalidSyntax(code.last_idx())),
        }
    }

    Ok(Node::UnionDec { name, variants })
}

fn parse_if(code: &mut Cursor) -> Result<Node, ParseError> {
    // if stuff == bleh:
    //     expression
    // elif otherstuff:
    //     expression
    // else:

    code.expect_ident()?;

    let pred = Box::new(parse_expr(code, 0)?);

    code.expect(Colon)?;
    code.expect(Newline)?;

    let block = Box::new(parse_block(code)?);

    // Weird syntax? Maybe rewrite.
    let else_block = if let Some(Ident(tok)) = code.peek() {
        match tok.as_str() {
            "else" => {
                code.next();
                code.expect(Colon)?;
                Some(Box::new(parse_block(code)?))
            }
            "elif" => Some(Box::new(parse_if(code)?)),
            _ => None,
        }
    } else {
        None
    };

    Ok(Node::If {
        pred: pred,
        block: block,
        else_block: else_block,
    })
}

// Really weird function, weird syntax, simple logic.
fn parse_type(code: &mut Cursor) -> Result<Node, ParseError> {
    Ok(Node::Type {
        name: match code.next() {
            Some(Ident(type_string)) => TypeType::Base(type_string),
            // ERROR: Multiple ref ('&&') turns into 'And' symbol in lexer.
            Some(Op(Operator::Ref)) => TypeType::Ref(Box::new(match parse_type(code)? {
                Node::Type { name } => name,
                _ => unreachable!(),
            })),
            _ => return Err(InvalidSyntax(code.last_idx())),
        },
    })
}

/*---Tests---*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_asn() {
        println!("Var Assignment:\n");
        let test = "var my_var: int = 0\nvar vartwo: &stuff";
        let thing = tokenize_code(test);
        println!("{:#?}", parse_file(thing, &"var_asn".to_string()));
    }

    #[test]
    fn test_quicksort_ast() {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::open("./examples/quicksort.zg").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        println!(
            "{:#?}",
            parse_file(tokenize_code(&contents), &"quicksort".to_string())
        )
    }
}
