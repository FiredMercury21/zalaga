use crate::ast::tree::ParseError::InvalidSyntax;

use super::lexer::*;
use super::lexer::TokType::*;
use super::lexer::Operator;


/*---Type Declarations---*/

/*
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
    Ref(Box<Prim>),
}
*/

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Num(i64),
    Float(f64),
} 

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Module{ name: String, root: Box<Node> },
    FnDec{  name: String, args: Vec<Node>, ret_type: String, body: Box<Node> },
    Block{               scope: Vec<Node> },
    FnCall{ name: String, args: Vec<Node> },
    Expr{                 expr: Box<Node> },
    VarAsn{ name: String,  val: Box<Node> },
    VarDec{ name: String, expr: Option<Box<Node>>, var_type: String },
    Var{    name: String },
    Ref{    expr: Box<Node> },
    Deref{  expr: Box<Node> },
    Match{  grds: Vec<Node> },
    Guard{  pred: Box<Node>, expr: Box<Node> },
    Field{  base: Box<Node>, field:    String },
    StructDef{ name: String, fields:   Vec<Node> },
    Struct{                  fields:   Vec<Node>},
    UnionDef{  name: String, fields:   Vec<Node>},
    Union{                   fields:   Vec<Node>},
    EnumDef{   name: String, variants: Vec<String>},
    Enum{   variant: String },
    For{    init: Box<Node>,  pred: Box<Node>, then: Box<Node>, block: Box<Node> },
    While{  pred: Box<Node>, block: Box<Node> },
    If{     pred: Box<Node>, block: Box<Node>, else_block: Option<Box<Node>> },
    BinOp{ first: Box<Node>,    op: Operator,      second: Box<Node> },
    UnOp{    val: Box<Node>,    op: Operator },
    Return{  val: Box<Node> },
    Const{   val: Constant },
    Use{    name: Box<Node> },
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
    EmptyFile,
    Generic,
}


// What we pass to every function.
// I wanted to use an iterator but there's a
// couple times we need to go back.
#[derive(Debug, Clone, PartialEq)]
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

    fn last_idx(&mut self) -> Span {
        // Ideally there should be checks on empty streams.
        // Usually we use this function after we read a bad token.
        match self.stream.get(self.pos - 1) {
            Some(tok) => tok.index.clone(),
            None => self.stream[self.pos - 2].index.clone()
        }
    }
}

/*---Helper functions---*/

// String to binary operator
fn is_bin_op(op: &Operator) -> bool {
    use Operator::*;
    matches!(op, Add | Sub | Mul | Div | Exp | Mod | LT | GT | ET | LorET | GorET | NotET | Or | And )
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
        Add   => 10,
        Sub   => 10, 
        Mul   => 15,   
        Div   => 15,   
        Exp   => 20,   
        Mod   => 15, 
        LT    => 5,    
        GT    => 5,    
        ET    => 5,    
        LorET => 5, 
        GorET => 5, 
        NotET => 5, 
        Or    => 5,    
        And   => 5,

        _ => return None
    })
}

// Don't think we use this... could be useful.
/*
fn is_end_key(c: &TokType) -> bool {
    use crate::ast::lexer::TokType::*;
    matches!(c, Eof | Guard | Comma | RBrack | RSquirl | SColon | Arrow | Indent | Dedent | Newline)
} 
*/

// Expect a certain token, else err.
macro_rules! expect_else_err {
    ($code:ident, $expected:pat, $ret:expr) => {
        let Some( Token { tok_type: $expected, index: _ } ) = $code.next() else {
            return Err($ret)
        };
    };
}

// Find appropriate parse function.
fn match_to_parse(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    match code.peek() { 
        Some(Token { tok_type: Ident(ident), .. }) => {
            Ok(match ident.as_str() {
                "fn"       => parse_fn_dec(code)?,
                "var"      => parse_var_dec(code)?,
                "enum"     => parse_enum_dec(code)?,
                "uni"      => parse_union_dec(code)?,
                "struct"   => parse_struct_dec(code)?,
                //"use"    => parse_use(code)?,
                "for"      => parse_for(code)?,
                "if"       => parse_if(code)?,
                "while"    => parse_while(code)?,
                "return"   => parse_return(code)?,
                "break"    => Node::Break,
                "continue" => Node::Continue,

                // If token after ident is =
                _ if matches!(code.stream.get(code.pos + 1), Some(Token { tok_type: Assign, .. })) => {
                    parse_var_asn(code)?
                },

                _ => parse_expr(code, 0)?,
            })
        },

        // If the thing is a pointer or in brackets, it's an expression.
        Some(Token { tok_type: Op(..), .. }) |
        Some(Token { tok_type: LBrack, ..}) => Ok(parse_expr(code, 0)?),

        Some(Token { tok_type: LSquirl, .. }) => Ok(parse_block(code)?),

        _ => return Err(InvalidSyntax(code.last_idx()))
    }

}

/*---Parsers---*/

pub fn parse_file(code: Vec<Token>, name: &String ) -> Result<Node, ParseError> {
    let mut cursor = Cursor { stream: code, pos: 0 };
    let root = Box::new(parse_block(&mut cursor)?);

    Ok(Node::Module {
        name: name.clone(),
        root: root
    })
}

// Blocks are whitespace-significant.
fn parse_block(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    let mut statements = Vec::new();

    if let Some(Token { tok_type: Indent, index: _ }) = code.peek() { code.next(); }

    while let Some(Token { tok_type, index }) = code.next() {
        match tok_type {
            Dedent | Eof => break,
            Newline => continue,
            Ident(_) => { code.pos -= 1; statements.push(match_to_parse(code)?) },
            _ => return Err(BlockParseErr(index.clone()))
        }
    }

    Ok(Node::Block { scope: statements })
}

fn parse_fn_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // fn name(arg1: type, arg2: type) -> ret_type {  }

    use ParseError::*;

    expect_else_err!(code, Ident(_),    InvalidSyntax(code.last_idx())); // Should never err.
    expect_else_err!(code, Ident(name), FnNoName(code.last_idx()));
    expect_else_err!(code, LBrack,      FnNoParen(code.last_idx()));

    let mut args = Vec::new();
    while let Some(Token { tok_type: Ident(arg), .. }) = code.next() {
        expect_else_err!(code, Colon,           VarNoType(code.last_idx()));
        expect_else_err!(code, Ident(type_str), VarNoType(code.last_idx()));
        
        // Have to do it manually here. Annoying.
        let check = code.next();
        let Some( Token { tok_type: Comma, .. } ) = check else {
            let Some( Token { tok_type: RBrack, .. } ) = check else {
                return Err(FnBadArg(code.last_idx()));
            };
            break;
        };

        args.push(Node::VarDec { name: arg, expr: None, var_type: type_str.clone() });
    }

    expect_else_err!(code, Arrow,           FnNoRetType(code.last_idx()));
    expect_else_err!(code, Ident(type_str), FnNoRetType(code.last_idx()));    
    expect_else_err!(code, Colon,           FnNoBody(   code.last_idx()));
    expect_else_err!(code, Indent,          FnNoBody(   code.last_idx()));
    let fn_body = parse_block(code)?;

    let body = Box::new(fn_body);
    let output = Node::FnDec { name: name, args: args, ret_type: type_str.clone(), body: body };

    Ok(output)
}

fn parse_var_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // var name: type
    // var name: type = stuff

    use ParseError::*;

    expect_else_err!(code, Ident(_),        InvalidSyntax(code.last_idx())); 
    expect_else_err!(code, Ident(name),     VarNoName(code.last_idx()));
    expect_else_err!(code, Colon,           VarNoType(code.last_idx()));
    expect_else_err!(code, Ident(type_str), VarNoType(code.last_idx()));

    let expr = if let Some(Token { tok_type: Assign, .. }) = code.peek() {
        code.next();
        Some(Box::new(parse_expr(code, 0)?))
    } else {
        None
    };

    Ok(Node::VarDec { name: name, expr: expr, var_type: type_str.clone() })
}

fn parse_fn_args(code: &mut Cursor) -> Result<Vec<Node>, ParseError> {
    // (arg1, arg2, arg3)

    use ParseError::*;

    expect_else_err!(code, LBrack, InvalidSyntax(code.last_idx())); 

    let mut args = Vec::new();
    while let Some(Token { tok_type: Ident(arg), .. }) = code.next() {   
        args.push(Node::Var { name: arg });     
        let check = code.next();
        let Some( Token { tok_type: Comma, .. } ) = check else {
            let Some( Token { tok_type: RBrack, .. } ) = check else {
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

    use ParseError::*;
    use Node::*;

    let Some(Token { tok_type, .. }) = code.next() else {
        return Err(ExprParseErr(code.last_idx()));
    };
    let mut current = match tok_type {
        Num(num) => Const { val: Constant::Num(num.parse().unwrap()) },
        LBrack => parse_expr(code, 0)?,
        LSquirl => parse_block(code)?,

        Op(op) if is_un_op(&op) => {
            let val = Box::new(parse_atom(code)?);
            UnOp { val, op }
        },

        Ident(name) if matches!(code.peek(), Some(Token { tok_type: LBrack, .. })) => {
            let args = parse_fn_args(code)?;
            FnCall { name, args }
        },

        _ => { code.pos -= 1; parse_atom(code)? }
    };

    loop {
        // Consumes last token. Should we?
        let Some(Token { tok_type: Op(op), .. }) = code.next() else { 
            break
        };
        if !is_bin_op(&op) { break }

        let new_prec = op_to_prec(&op).unwrap();
        if new_prec < prec { 
            break 
        }
        let second = Box::new(parse_expr(code, new_prec + 1)?);

        current = BinOp { 
            first: Box::new(current), 
            op: op, 
            second: second
        };
    }

    Ok(current)
}

fn parse_atom(code: &mut Cursor) -> Result<Node, ParseError> {
    // Ref and deref have Ident after them unless bracks,
    // In which case expression.
    // Field access can follow 
    use ParseError::*;
    use Node::*;

    let current = if let Some(Token { tok_type, .. }) = code.next() {
        match tok_type {
            Deref => Deref { expr: Box::new(match code.peek() {
                Some(Token { tok_type: LBrack,      .. }) => parse_expr(code, 0)?,
                Some(Token { tok_type: Ident(name), .. }) => Var { name },
                _ => return Err(InvalidSyntax(code.last_idx()))
            }) },

            Ref => Ref { expr: Box::new(match code.peek() {
                Some(Token { tok_type: LBrack,      .. }) => parse_expr(code, 0)?,
                Some(Token { tok_type: Ident(name), .. }) => Var { name },
                _ => return Err(InvalidSyntax(code.last_idx()))
            }) },

            Ident(name) => Var { name },

            LBrack => {
                let expr = Box::new(parse_expr(code, 0)?);
                Expr { expr }
            },

            _ => return Err(InvalidSyntax(code.last_idx()))
        }
    } else {
        return Err(InvalidSyntax(code.last_idx()))
    };

    while matches!(code.peek(), Some(Token { tok_type: Period, .. })) {
        expect_else_err!(code, Ident(field), InvalidSyntax(code.last_idx()));
        current = Field { 
            base: Box::new(current), 
            field
        };
    }

    return current
}

fn parse_for(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    // for (
    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));
    expect_else_err!(code, LBrack,     InvalidSyntax(code.last_idx()));

    // var i: int = 0; i < 12; ++i
    let init  = Box::new(parse_var_dec(code)?);
    let pred  = Box::new(parse_expr(code, 0)?);
    let then  = Box::new(parse_expr(code, 0)?);    

    // ):
    expect_else_err!(code, RBrack, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Colon,  InvalidSyntax(code.last_idx()));

    let block = Box::new(parse_block(code)?);

    Ok(Node::For { 
        init: init, 
        pred: pred,
        then: then,
        block: block
    })
}

fn parse_while(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));

    let pred  = Box::new(parse_expr(code, 0)?);
    let block = Box::new(parse_block(code)?);

    Ok(Node::While { 
        pred: pred, 
        block: block
    })
}

fn parse_match(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));    
    expect_else_err!(code, Colon,      InvalidSyntax(code.last_idx())); 
    expect_else_err!(code, Indent,     InvalidSyntax(code.last_idx())); 

    let mut guards = Vec::new();
    while let Some(Token { tok_type: Guard, .. }) = code.next() {   
        let pred = Box::new(parse_expr(code, 0)?);
        expect_else_err!(code, Arrow, InvalidSyntax(code.last_idx())); 
        let expr = Box::new(parse_expr(code, 0)?);
        guards.push(Node::Guard { pred: pred, expr: expr });

        let check = code.next();
        let Some( Token { tok_type: Comma, .. } ) = check else {
            let Some( Token { tok_type: Dedent, .. } ) = check else {
                return Err(FnBadArg(code.last_idx()));
            };
            break;
        };
    }

    Ok(Node::Match { grds: guards })
}

fn parse_var_asn(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(name), InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Assign,      InvalidSyntax(code.last_idx()));

    let val = Box::new(parse_expr(code, 0)?);

    Ok(Node::VarAsn { 
        name: name, 
        val: val
    })
}

fn parse_return(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));

    let val = Box::new(parse_expr(code, 0)?);

    Ok(Node::Return { val: val })
}

fn parse_enum_def(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Ident(name), InvalidSyntax(code.last_idx()));    
    expect_else_err!(code, Colon, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Newline, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Indent, InvalidSyntax(code.last_idx()));
}

fn parse_struct_def(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Ident(name), InvalidSyntax(code.last_idx()));    
    expect_else_err!(code, Colon, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Newline, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Indent, InvalidSyntax(code.last_idx()));
}

fn parse_union_def(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Ident(name), InvalidSyntax(code.last_idx()));    
    expect_else_err!(code, Colon, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Newline, InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Indent, InvalidSyntax(code.last_idx()));
}

fn parse_if(code: &mut Cursor) -> Result<Node, ParseError> {
    // if stuff == bleh:
    //     expression
    // elif otherstuff:
    //     expression
    // else:

    use ParseError::*;

    expect_else_err!(code, Ident(_), InvalidSyntax(code.last_idx()));

    let pred = Box::new(parse_expr(code, 0)?);

    expect_else_err!(code, Colon,   InvalidSyntax(code.last_idx()));
    expect_else_err!(code, Newline, InvalidSyntax(code.last_idx()));

    let block = Box::new(parse_block(code)?);

    // Weird syntax? Maybe rewrite.
    let else_block = if let Some(Token { tok_type: Ident(tok), ..}) = code.peek() {
        match tok.as_str() {
            "else" => { 
                code.next();     
                expect_else_err!(code, Colon,   InvalidSyntax(code.last_idx()));
                Some(Box::new(parse_block(code)?))
            },
            "elif" => Some(Box::new(parse_if(code)?)),
            _ => None
        }
    } else {
        None
    };

    Ok(Node::If { pred: pred, block: block, else_block: else_block })
}

/*---Tests---*/

#[cfg(test)]
mod tests {
    use crate::ast::lexer::*;
    use super::*;

    #[test]
    fn test_var_asn() {
        println!("Var Assignment:\n");
        let test = "var my_var: int = 0";
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
        println!("{:#?}", parse_file(tokenize_code(&contents), &"quicksort".to_string()))
    }
}
