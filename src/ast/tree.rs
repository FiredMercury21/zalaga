use super::ast_types::ExprType::*;
use super::ast_types::NodeType::*;
use super::ast_types::ParseErrorType::*;
use super::ast_types::*;

use super::lexer::TokType::*;
use super::lexer::*;

/*---Types---*/

// All types are present within the `ast_types.rs`.

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
    matches!(op, Neg | Inc | Dec | Ref | Deref)
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
        Assign => 3,

        _ => return None,
    })
}

// Find appropriate parse function.
fn match_to_parse(code: &mut Cursor) -> Result<Node, ParseError> {
    Ok(match code.peek() {
        Some(Ident(ident)) => {
            match ident.as_str() {
                "fn" => parse_fn_dec(code)?,
                "var" => parse_var_dec(code)?,
                "enum" => parse_enum_dec(code)?,
                "struct" => parse_struct_dec(code)?,
                //"use"  => parse_use(code)?,
                "for" => parse_for(code)?,
                "while" => parse_while(code)?,

                _ => {
                    let expr = parse_expr(code, 0)?;
                    code.new_node(Statement { expr })
                }
            }
        }

        // If the thing is a pointer or in brackets, it's an expression.
        Some(Op(..) | LBrack) => {
            let expr = parse_expr(code, 0)?;
            code.new_node(Statement { expr })
        }

        Some(Indent | LSquirl) => {
            let expr = parse_block(code)?;
            code.new_node(Statement { expr })
        }

        _ => {
            return Err(ParseError {
                err: InvalidKeyword,
                span: code.last_idx(),
            });
        }
    })
}

/*---Parsers---*/

pub fn parse_file(code: Vec<Token>, name: &str) -> Result<Node, ParseError> {
    let mut cursor = Cursor {
        stream: code,
        pos: 0,
        node_id: Id(0),
    };

    let mut scope = Vec::new();

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
            _ => scope.push(match_to_parse(&mut cursor)?),
        }
    }

    Ok(cursor.new_node(Module {
        name: name.to_owned(),
        scope,
    }))
}

// Blocks are whitespace-significant.
fn parse_block(code: &mut Cursor) -> Result<Expr, ParseError> {
    let mut statements = Vec::new();

    if let Some(Indent) = code.peek() {
        code.next();
    }

    while let Some(token) = code.peek() {
        match token {
            Dedent | Eof | RSquirl => {
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

    Ok(code.new_expr(ExprType::Block { scope: statements }))
}

fn parse_fn_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    // fn name(arg1: type, arg2: type) -> ret_type {  }

    code.expect_ident()?; // Should never err.
    let name = code.expect_ident_else(FnNoName)?;
    code.expect_else(LBrack, FnNoParen)?;

    let mut args = Vec::new();
    loop {
        if Some(RBrack) == code.peek() {
            break;
        }
        let arg = code.expect_ident()?;
        code.expect_else(Colon, VarNoType)?;
        let var_type = Box::new(parse_type(code)?);
        if Some(Comma) == code.peek() {
            code.next();
        }
        args.push(code.new_node(VarDec {
            name: arg,
            expr: None,
            var_type,
        }));
    }

    code.expect_else(RBrack, FnNoParen)?;
    code.expect_else(Arrow, FnNoRetType)?;
    let Ok(ret_type) = parse_type(code) else {
        return Err(ParseError {
            err: FnNoRetType,
            span: code.last_idx(),
        });
    };
    let ret_type = Box::new(ret_type);
    code.expect_else(Colon, FnNoRetType)?;
    code.expect_else(Newline, FnSyntax)?;
    code.expect_else(Indent, FnSyntax)?;

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
    let name = code.expect_ident_else(VarNoName)?;
    code.expect_else(Colon, VarNoType)?;
    let var_type = Box::new(parse_type(code)?);

    let expr = if let Some(Op(Operator::Assign)) = code.peek() {
        code.next();
        Some(parse_expr(code, 0)?)
    } else {
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
            _ => {
                return Err(ParseError {
                    err: FnBadArg,
                    span: code.last_idx(),
                });
            }
        }
    }
    code.expect_else(RBrack, FnNoCloseBrack)?;

    Ok(args)
}

// TODO: Add match arms for keywords like match, if, etc.
// TODO: Treat a Sub at the start as a negative sign. var a: int = -3.
fn parse_expr(code: &mut Cursor, prec: i32) -> Result<Expr, ParseError> {
    // func( (a / 2), 3);
    // 1 + 1;
    // ( func(arg1, arg2) + x ) * y;
    // mystruct[ field1 = func(x); field2 = 2 + 3 ].field2 + 5 == 10

    // This is a tough one. Expressions can be recursive.

    let Some(token) = code.next() else {
        return Err(ParseError {
            err: ExprParseErr,
            span: code.last_idx(),
        });
    };
    let mut current = match token {
        // Constant numbers.
        Num(num) => code.new_expr(Const {
            val: Constant::Num(num.parse().unwrap()),
        }),

        // Bracketed expressions.
        LBrack => {
            let expr = parse_expr(code, 0)?;
            code.expect_else(RBrack, UnclosedBrack)?;
            expr
        }

        // Block expressions.
        LSquirl => parse_block(code)?,

        // Unary operators.
        Op(op) if is_un_op(&op) => {
            let expr = Box::new(parse_expr(code, 0)?);
            code.new_expr(UnOp { op, expr })
        }

        // If statement
        Ident(key) if key == "if" => {
            code.pos -= 1; // Don't like this.
            parse_if(code)?
        }

        // Break statement
        Ident(key) if key == "break" => {
            code.pos -= 1;
            parse_break(code)?
        }

        // Return statement
        Ident(key) if key == "return" => {
            code.pos -= 1;
            parse_return(code)?
        }

        // Match statement
        //Ident(key) if key == "match" => { code.pos -= 1; parse_match(code)? },

        // Function calls.
        Ident(name) if matches!(code.peek(), Some(LBrack)) => {
            let args = parse_fn_args(code)?;
            code.new_expr(FnCall { name, args })
        }

        // Struct def.
        Ident(name) if matches!(code.peek(), Some(LSquare)) => {
            let fields = parse_struct(code)?;
            code.new_expr(Struct { name, fields })
        }

        // Enum variant.
        // Maybe I should make parse_expr(). Prob not.
        Ident(variant) if matches!(code.peek(), Some(At)) => {
            // variant@myenum[ val ]
            // emptyvar@myenum

            code.expect(At)?;
            let name = code.expect_ident()?;
            let val = if code.peek() == Some(LSquare) {
                code.next();
                let payload = Box::new(parse_expr(code, 0)?);
                code.expect(RSquare)?;
                Some(payload)
            } else {
                None
            };

            code.new_expr(Enum { name, variant, val })
        }

        Ident(name) => code.new_expr(Var { name }),

        _ => {
            return Err(ParseError {
                err: BadExpr,
                span: code.last_idx(),
            });
        }
    };

    loop {
        // Field access. Duplicates parse_atom, because that doesn't work on structs. Refactor?
        while matches!(code.peek(), Some(Period)) {
            code.next();
            let field = code.expect_ident_else(InvalidField)?;
            current = code.new_expr(Field {
                base: Box::new(current),
                field,
            });
        }

        let Some(Op(op)) = code.peek() else { break };
        if !is_bin_op(&op) {
            break;
        }

        let new_prec = op_to_prec(&op).unwrap();
        if new_prec < prec {
            break;
        }
        code.next();
        let second = Box::new(parse_expr(code, new_prec + 1)?);

        current = code.new_expr(BinOp {
            first: Box::new(current),
            op,
            second,
        });
    }

    Ok(current)
}

// TODO: Match.
// Those are related. How to handle enum syntax?

fn parse_for(code: &mut Cursor) -> Result<Node, ParseError> {
    // for (
    code.expect_ident()?;
    code.expect(LBrack)?;

    // var i: int = 0; i < 12; ++i
    let init = Box::new(parse_var_dec(code)?);
    code.expect_else(SColon, ForNoInit)?;
    let pred = parse_expr(code, 0)?;
    code.expect_else(SColon, ForNoPred)?;
    let then = parse_expr(code, 0)?;

    // ):
    code.expect_else(RBrack, UnclosedBrack)?;
    code.expect_else(Colon, ForNoBlock)?;

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
    code.expect_else(Colon, WhileNoBlock)?;
    let block = parse_block(code)?;

    Ok(code.new_node(While { pred, block }))
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

fn parse_break(code: &mut Cursor) -> Result<Expr, ParseError> {
    code.expect_ident()?;

    let val = match code.peek() {
        // What other tokens mean no expr?
        Some(Newline) => None,
        _ => Some(Box::new(parse_expr(code, 0)?)),
    };

    Ok(code.new_expr(Break { val }))
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
    code.expect_else(Colon, EnumNoBlock)?;
    code.expect_else(Newline, EnumNoBlock)?;
    code.expect_else(Indent, EnumNoBlock)?;
    let mut variants = Vec::new();
    loop {
        variants.push(EnumVariant {
            name: code.expect_ident()?,
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

            _ => {
                return Err(ParseError {
                    err: EnumBadSyntax,
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(code.new_node(EnumDec { name, variants }))
}

fn parse_struct_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect_else(Colon, StructNoBlock)?;
    code.expect_else(Newline, StructNoBlock)?;
    code.expect_else(Indent, StructNoBlock)?;

    let mut fields = Vec::new();
    while let Some(Ident(field)) = code.next() {
        code.expect(Colon)?;
        let var_type = Box::new(parse_type(code)?);
        fields.push(code.new_node(VarDec {
            name: field,
            expr: None,
            var_type,
        }));
        match code.next() {
            Some(Newline) => {
                code.expect(Dedent)?;
                break;
            }

            Some(Comma) => {
                code.expect(Newline)?;
                continue;
            }

            _ => {
                return Err(ParseError {
                    err: StructBadSyntax,
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(code.new_node(StructDec { name, fields }))
}

fn parse_struct(code: &mut Cursor) -> Result<Vec<Expr>, ParseError> {
    code.expect(LSquare)?;
    let mut fields = Vec::new();
    while let Some(Ident(field)) = code.next() {
        code.expect_else(Op(Operator::Assign), StructNoFieldInit)?;
        let val = parse_expr(code, 0)?;
        let field_var = code.new_expr(Var { name: field });
        fields.push(code.new_expr(BinOp {
            first: Box::new(field_var),
            op: Operator::Assign,
            second: Box::new(val),
        }));
        match code.next() {
            Some(RSquare) => {
                code.expect(Newline)?;
                break;
            }

            Some(Comma) => continue,

            _ => {
                return Err(ParseError {
                    err: StructBadSyntax,
                    span: code.last_idx(),
                });
            }
        }
    }

    Ok(fields)
}

fn parse_if(code: &mut Cursor) -> Result<Expr, ParseError> {
    // if stuff == bleh:
    //     expression
    // elif otherstuff:
    //     expression
    // else:

    code.expect_ident()?;

    let pred = Box::new(parse_expr(code, 0)?);

    code.expect_else(Colon, IfNoBlock)?;
    code.expect_else(Newline, IfNoBlock)?;
    code.expect_else(Indent, IfNoBlock)?;

    let then = Box::new(parse_block(code)?);

    // Weird syntax? Maybe rewrite.
    let else_block = if let Some(Ident(tok)) = code.peek() {
        match tok.as_str() {
            "else" => {
                code.next();
                code.expect_else(Colon, IfNoBlock)?;
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
                    match code.next() {
                        Some(Ident(type_string)) => break Base(type_string),
                        Some(Op(Operator::Ref)) => ref_n += 1,
                        // Ewww. To handle '&&' turning into 'And' in lexer.
                        Some(Op(Operator::And)) => ref_n += 2,
                        _ => {
                            return Err(ParseError {
                                err: BadType,
                                span: code.last_idx(),
                            });
                        }
                    }
                };
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

    #[test]
    fn test_var_asn() {
        let test = "var my_var: int = 0\nvar vartwo: &stuff";
        let thing = tokenize_code(test);
        assert!(parse_file(thing, &"var_asn".to_string()).is_ok());
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
            parse_file(tokenize_code(&contents), &"quicksort".to_string()).unwrap()
        );
        assert!(parse_file(tokenize_code(&contents), &"quicksort".to_string()).is_ok());
    }
}
