use super::lexer::*;
use super::lexer::TokType::*;


/*---Type Declarations---*/

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,   // +
    Sub,   // -
    Mul,   // *
    Div,   // /
    Exp,   // ^
    FDiv,  // //
    Mod,   // %
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
    Arr(Box<Prim>),
    
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Num(i64),
    Float(f64),
    Bool(bool),
    String(String),
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
    Match{  grds: Vec<Node> },
    Guard{  pred: Box<Node>,  expr: Box<Node> },
    For{    init: Box<Node>,  pred: Box<Node>, then: Box<Node>, block: Box<Node> },
    While{  pred: Box<Node>, block: Box<Node> },
    If{     then: Box<Node>,  expr: Box<Node>, else_block: Box<Node> },
    BinOp{ first: Box<Node>,    op: Operator,      second: Box<Node> },
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
    EmptyFile,
    Generic,
}


// What we pass to every function.
// I wanted to use an iterator but there's a
// couple times we need to go back.
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
        // Usually we use this function after we read a bad token,
        // So it's better to go two back to find a good one.
        self.stream[self.pos - 2].index.clone()
    }
}

/*---Helper functions---*/

// String to binary operator
fn str_to_b_op(input_str: &str) -> Option<Operator> {
    use Operator::*;

    Some(match input_str {
        "+"  => Add,
        "-"  => Sub, 
        "*"  => Mul,   
        "/"  => Div,
        "^"  => Exp,   
        "//" => FDiv,       
        "%"  => Mod,
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

// String to unary operator
fn str_to_u_op(input_str: &str) -> Option<Operator> {
    use Operator::*;

    Some(match input_str {
        "!"  => Neg,   
        "++" => Inc,   
        "--" => Dec, 

        _ => return None
    })
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
        FDiv  => 15,
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

// String to primitive type
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
        "()"     => Void, // Should we use never?

        x if is_arr(x) => Arr(Box::new(str_to_prim(inner(x)?)?)),

        _ => return None
    })
}

// Don't think we use this... could be useful.
fn is_end_key(c: &TokType) -> bool {
    use crate::ast::lexer::TokType::*;
    matches!(c, Eof | Guard | Comma | RBrack | RSquirl | SColon | Arrow | Indent | Dedent | Newline)
}

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

    // Better be an ident! What lines of code don't start with ident?
    // I guess some expressions. Might have something for that.
    let tok = code.peek();
    let Some(Token { tok_type: Ident(ident), .. }) = tok else { 
        return Err(InvalidSyntax(code.last_idx()))
    };

    Ok(match ident.as_str() {
        "fn"     => parse_fn_dec(code)?,
        "var"    => parse_var_dec(code)?,
        //"enum"   => parse_enum_dec(code)?,
        //"uni"    => parse_union_dec(code)?,
        "for"    => parse_for(code)?,
        "while"  => parse_while(code)?,
        "return" => parse_return(code)?,
        //"break"  => 
        //"continue" =>

        // If token after ident is =
        _ if matches!(code.stream.get(code.pos + 1), Some(Token { tok_type: Assign, .. })) => {
            parse_var_asn(code)?
        },

        _ => parse_expr(code, 0)?,
    })
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

fn parse_block(code: &mut Cursor) -> Result<Node, ParseError> {
    use ParseError::*;

    let mut statements = Vec::new();

    if let Some(Token { tok_type: Indent, index: _ }) = code.peek() { code.next(); }

    while let Some(tok) = code.peek() {
        match &tok.tok_type {
            Dedent => break,
            Eof => break,
            Ident(_) => statements.push(match_to_parse(code)?),
            _ => return Err(BlockParseErr(tok.index.clone()))
        }
    }

    Ok(Node::Block { scope: statements })
}

// Horrendous code. May God forgive me.
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
    // func( {a / 2}, 3);
    // 1 + 1;
    // { func(arg1, arg2) + x } * y;

    // This is a tough one. Expressions can be recursive.

    use ParseError::*;
    use Node::*;

    let Some(Token { tok_type: token, .. }) = code.next() else {
        return Err(ExprParseErr(code.last_idx()));
    };
    let mut current = match token {
        Num(num) => Const { val: Constant::Num(num.parse().unwrap()) },
        Str(string) => Const { val: Constant::String(string) },
        LBrack => parse_expr(code, 0)?,
        Indent => { code.pos -= 1; parse_block(code)? },

        Ident(c) if str_to_u_op(&c).is_some() => {
            UnOp { val: Box::new(parse_expr(code, 0)?), op: str_to_u_op(&c).unwrap() }
        },

        Ident(name) if matches!(code.peek(), Some(Token { tok_type: LBrack, .. })) => {
            FnCall { name: name, args: parse_fn_args(code)? }
        },

        Ident(name) => {
            Var { name: name }
        },

        _ => return Err(ExprParseErr(code.last_idx()))
    };

    loop {
        let Some(Token { tok_type: Ident(c), .. }) = code.next() else { 
            break
        };
        let Some(op) = str_to_b_op(&c) else {
            break
        };

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
}