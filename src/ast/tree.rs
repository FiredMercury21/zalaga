use super::lexer::*;
use super::lexer::TokType::*;
use std::iter::Peekable;

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

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Module{ name: String, root: Box<Node> },
    FnDef{  name: String, args: Vec<Node>,  ret_type: Prim, body: Box<Node> },
    Block{               scope: Vec<Node>,  ret_type: Prim },
    FnCall{ name: String, args: Vec<Node>,  ret_type: Prim },
    Expr{                 expr: Vec<Node>,  ret_type: Prim },
    VarDec{ name: String,                   var_type: Prim },
    Var{    name: String,                   var_type: Prim },
    Match{  grds: Vec<Node> },
    Guard{  pred: Box<Node>,  expr: Box<Node> },
    For{    pred: Box<Node>, block: Box<Node> },
    While{  pred: Box<Node>, block: Box<Node> },
    If{     then: Box<Node>,  expr: Box<Node>, else_block: Box<Node> },
    BinOp{  frst: Box<Node>,    op: Operator,      second: Box<Node> },
    UnOp{    val: Box<Node>,    op: Operator }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    FnNoRetType(Span),
    FnNoParen(Span),
    FnNoName(Span),
    FnNoBody(Span),
    FnBadArg(Span),
    VarNoType(Span),
    Generic
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

fn construct_block_ast<T>(tokens: &mut Peekable<T>) -> Result<Node, ParseError> 
where T: Iterator<Item = Token> 
{
    /*
    while let Some(tok) = tokens.peek() {
        match tok.tok_type {
            RSquirl => break,
            Eof => break,
            Ident(ident) => {
                match ident {
                    "fn".as_string() => parse_fn_dec(tokens)?,

                }
            },
        }
    }
    */
    todo!();
}

macro_rules! expect_else_err {
    ($code:ident, $expected:pat, $idx:ident, $ret:expr) => {
        let Some( Token { tok_type: $expected, index: index_match } ) = $code.next() else {
            return Err($ret)
        };
        let $idx = index_match;
    };
}

// Horrendous code. May God forgive me.
fn parse_fn_dec<T>(code: &mut Peekable<T>) -> Result<Node, ParseError> 
where T: Iterator<Item = Token>
{
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

        args.push(Node::VarDec { name: arg, var_type: var_type });
    }

    expect_else_err!(code, Arrow, idx, FnNoRetType(Span { line: idx.line, idx: idx.idx }));
    expect_else_err!(code, Ident(type_str),                     idx, FnNoRetType(Span { line: idx.line, idx: idx.idx }));
    expect_else_err!(code, LSquirl,                             idx, FnNoBody(   Span { line: idx.line, idx: idx.idx }));
    let fn_body = construct_block_ast(code)?;

    let Some(ret_type) = str_to_prim(&type_str) else { 
        return Err(FnNoRetType(Span { line: idx.line, idx: idx.idx }))
    };
    let output = Node::FnDef { name: name, args: args, ret_type: ret_type, body: Box::new(fn_body) };

    Ok(output)
}

pub fn construct_ast(tokens: Vec<Token>, mod_name: &String) -> Result<Node, ParseError> {
    let mut look = tokens.into_iter().peekable();
    let root = construct_block_ast(&mut look)?;
    Ok(Node::Module { name: mod_name.clone(), root: Box::new(root) })
}