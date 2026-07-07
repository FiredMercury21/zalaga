use super::lexer::*;
use super::lexer::TokType::*;
use std::iter::Peekable;


/*---Type Declarations---*/

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,   // +
    Sub,   // -
    Mul,   // *
    Div,   // /
    Exp,   // ^
    FDiv,  // //
    LT,    // <
    GT,    // >
    ET,    // ==
    LorET, // <=
    GorET, // >=
    NotET, // !=
    Or,    // ||
    And,   // &&
    Neg,   // !
    Inc,   // ++
    Dec,   // --
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Char,
    Int16,
    Int32,
    Int64,
    Float16,
    Float32,
    Float64,
    Bool,
    String,
    Void,
    Arr(Box<Prim>)
}

pub enum Constant {
    Num(i64),
    Float(f64),
    Bool(bool),
    String(String),
} 

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Module{ name: String, root: Box<Node> },
    FnDec{  name: String, args: Vec<Node>, ret_type: Prim, body: Box<Node> },
    Block{               scope: Vec<Node> },
    FnCall{ name: String, args: Vec<Node> },
    Expr{                 expr: Box<Node> },
    VarAsn{ name: String,  val: Box<Node> },
    VarDec{ name: String, expr: Option<Box<Node>>, var_type: Prim },
    Var{    name: String },
    Match{  grds: Vec<Node> },
    Guard{  pred: Box<Node>,  expr: Box<Node> },
    For{    pred: Box<Node>, block: Box<Node> },
    While{  pred: Box<Node>, block: Box<Node> },
    If{     then: Box<Node>,  expr: Box<Node>, else_block: Box<Node> },
    BinOp{  frst: Box<Node>,    op: Operator,      second: Box<Node> },
    UnOp{    val: Box<Node>,    op: Operator },
    Return{  val: Box<Node> },
    Const{   val: Constant },
    Break,
    Continue,
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
    Generic,
}

// What we pass to every function.
struct Cursor {
    stream: Vec<Token>,
    pos: usize
}

impl Iterator for Cursor {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        let ret = self.stream.get(self.pos).cloned();
        self.pos += 1;
        ret
    }
}

impl Cursor {
    fn peek(&mut self) -> Option<Token> {
        self.stream.get(self.pos).cloned()
    }
}

/*---Helper functions---*/

fn str_to_b_op(input_str: &str) -> Option<Operator> {
    use Operator::*;

    Some(match input_str {
        "+"  => Add,
        "-"  => Sub, 
        "*"  => Mul,   
        "/"  => Div,   
        "^"  => Exp,   
        "//" => FDiv,  
        "<"  => LT,    
        ">"  => GT,    
        "==" => ET,    
        "<=" => LorET, 
        ">=" => GorET, 
        "!=" => NotET, 
        "||" => Or,    
        "&&" => And,   

        _ => return None
    })
}

fn str_to_u_op(input_str: &str) -> Option<Operator> {
    use Operator::*;

    Some(match input_str {
        "!"  => Neg,   
        "++" => Inc,   
        "--" => Dec, 

        _ => return None
    })
}  

fn str_to_prim<'a>(input_str: &'a str) -> Option<Prim> {
    use Prim::*;
        
    let is_arr = |input: &str| input.starts_with('[') && input.ends_with(']');
    let inner  = |input: &'a str| Some(input.strip_prefix('[')?.strip_suffix(']')?.trim());

    Some(match input_str {
        "char"   => Char,
        "short"  => Int16,
        "int"    => Int32,
        "long"   => Int64,
        "half"   => Float16,
        "float"  => Float32,
        "double" => Float64,
        "bool"   => Bool,
        "str"    => String,
        "()"     => Void,

        x if is_arr(x) => Arr(Box::new(str_to_prim(inner(x)?)?)),

        _ => return None
    })
}

pub fn is_end_key(c: &TokType) -> bool {
    use crate::ast::lexer::TokType::*;
    matches!(c, Eof, Guard, Comma, RBrack, RSquirl, SColon, Arrow, Indent, Newline)
}

macro_rules! expect_else_err {
    ($code:ident, $expected:pat, $idx:ident, $ret:expr) => {
        let Some( Token { tok_type: $expected, index: index_match } ) = $code.next() else {
            return Err($ret)
        };
        let $idx = index_match;
    };
}

fn match_to_parse(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    let tok = code.peek();
    let Some(Token { tok_type: Ident(ident), index: idx }) = tok else { return Err(Generic) };

    Ok(match ident.as_str() {
        "fn"     => parse_fn_dec(code)?,
        "var"    => parse_var_dec(code)?,
        "for"    => parse_for(code)?,
        "while"  => parse_while(code)?,
        "return" => parse_return(code)?,

        // Horrendous. 
        ident if { 
            code.next(); code.peek() == Token { tok_type: Colon, index: _ }
        } => {
            parse_var_dec(code)?
        },

        ident if { 
            code.next(); code.peek() == Token { tok_type: Assign, index: _ }
        } => {
            code.pos -= 1;
            parse_var_asn(code)?
        },

        ident => parse_expr(code, 0)?,
    })
}

/*---Parsers---*/

fn parse_block(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    let mut statements = Vec::new();
    let mut idx: usize;

    expect_else_err!(code, LSquirl, idx, Generic);

    while let Some(tok) = code.peek() {
        match &tok.tok_type {
            RSquirl => break,
            Eof => break,
            Ident(ident) => statements.push(match_to_parse(code)?),
            _ => { return Err(BlockParseErr(tok.index.clone())) }
        }
    }

    Ok(Node::Block { scope: statements })
}

// Horrendous code. May God forgive me.
fn parse_fn_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // fn name(arg1: type, arg2: type) -> ret_type {  }

    use ParseError::*;
    let mut idx: Span;

    expect_else_err!(code, fnkey,       idx, Generic); // Should never err.
    expect_else_err!(code, Ident(name), idx, FnNoName(idx));
    expect_else_err!(code, LParen,      idx, FnNoParen(idx));

    let mut args = Vec::new();
    while let Some(Token { tok_type: Ident(arg), index: idx }) = code.next() {
        expect_else_err!(code, Colon,           idx, VarNoType(Span { line: idx.line, idx: idx.idx + arg.len() }));
        expect_else_err!(code, Ident(type_str), idx, VarNoType(Span { line: idx.line, idx: idx.idx             }));

        let Some(var_type) = str_to_prim(&type_str) else { 
            return Err(VarNoType(Span { line: idx.line, idx: idx.idx}))
        };
        
        // Have to do it manually here. Annoying.
        let check = code.next();
        let Some( Token { tok_type: Comma, index: idx } ) = check else {
            let Some( Token { tok_type: RParen, index: idx } ) = check else {
                return Err(FnBadArg(Span { line: idx.line, idx: idx.idx }));
            };
            break;
        };

        args.push(Node::VarDec { name: arg, expr: None, var_type: var_type });
    }

    expect_else_err!(code, Arrow,           idx, FnNoRetType(Span { line: idx.line, idx: idx.idx }));
    expect_else_err!(code, Ident(type_str), idx, FnNoRetType(Span { line: idx.line, idx: idx.idx }));
    expect_else_err!(code, LSquirl,         idx, FnNoBody(   Span { line: idx.line, idx: idx.idx }));
    let fn_body = parse_block(code)?;

    let Some(ret_type) = str_to_prim(&type_str) else { 
        return Err(FnNoRetType(Span { line: idx.line, idx: idx.idx }))
    };
    let output = Node::FnDec { name: name, args: args, ret_type: ret_type, body: Box::new(fn_body) };

    Ok(output)
}

fn parse_var_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // var name: type;
    // var name: type = stuff;

    use ParseError::*;
    let mut idx: Span;

    expect_else_err!(code, varkey,          idx, Generic); // Should never err.
    expect_else_err!(code, Ident(name),     idx, VarNoName(idx));
    expect_else_err!(code, Colon,           idx, VarNoType(idx));
    expect_else_err!(code, Ident(type_str), idx, VarNoType(idx));

    let Some(var_type) = str_to_prim(&type_str) else { 
        return Err(VarNoType(Span { line: idx.line, idx: idx.idx }))
    };

    let Some(tok) = code.next() else { return Err(Generic) };
    let expr = match tok.tok_type {
        Newline => None,
        Assign  => Some(Box::new(parse_expr(code, 0)?)),
        _ => { return Err(Generic) }
    };

    let output = Node::VarDec { name: name, expr: expr, var_type: var_type };

    Ok(output)
}

fn parse_fn_args(code: &mut Cursor) -> Result<Vec<Node>, ParseError> {
    // (arg1, arg2, arg3)

    use ParseError::*;
    let mut idx: Span;

    expect_else_err!(code, LParen, idx, Generic); // Should never err.

    let mut args = Vec::new();
    while let Some(Token { tok_type: Ident(arg), index: idx }) = code.next() {   
        args.push(Node::Var { name: arg });     
        let check = code.next();
        let Some( Token { tok_type: Comma, index: idx } ) = check else {
            let Some( Token { tok_type: RParen, index: idx } ) = check else {
                return Err(FnBadArg(Span { line: idx.line, idx: idx.idx }));
            };
            break;
        };
    }

    Ok(args)
}

fn parse_expr(code: &mut Cursor, prec: i32) -> Result<Node, ParseError> {
    // func( {a / 2}, 3);
    // 1 + 1;
    // { func(arg1, arg2) + x } * y;

    // This is a tough one. Expressions can be recursive.

    use ParseError::*;
    use Node::*

    let origin = code.pos;  
    let first = match code.next() {
        Num(num) => Const(Constant::Num(num)),
        Str(string) => Const(Constant::String(string)),
        LParen => parse_expr(code, 0)?,
        LBrack => { code.pos -= 1; parse_block(code)? },

        Ident(c) if str_to_u_op(c).is_some() => {
            UnOp { val: Box::new(parse_expr(code, 0)?), op: str_to_u_op(c).unwrap() }
        },

        Ident(name) if { code.peek() == Token { tok_type: LParen, index: _ } } => {
            FnCall { name: name, args: parse_fn_args(code)? }
        },

        Ident(name) => {
            Var { name: name }
        },

        _ => { return Err(ExprParseErr(code.stream[code.pos - 1].index)) }
    }

    match code.next() {
        Ident(c) if str_to_b_op(c).is_some() => {
            let op = str_to_b_op(c).unwrap();
            if op_to_prec(op) < prec {
                // THIS IS HARD
                todo!();
            } else {
                BinOp { first: first, op: op, second: parse_expr(code, op_to_prec(c))?}
            }
        },
        _ => first
    }
}

fn parse_for(code: &mut Cursor) -> Result<Node, ParseError> {
    todo!();
}

fn parse_while(code: &mut Cursor) -> Result<Node, ParseError> {
    todo!();
}

fn parse_match(code: &mut Cursor) -> Result<Node, ParseError> {
    todo!();
}

fn parse_var_asn(code: &mut Cursor) -> Result<Node, ParseError> {
    todo!();
}