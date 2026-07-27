use super::ast_types::ExprKind::*;
use super::ast_types::NodeKind::*;
use super::ast_types::ParseErrorKind::*;
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
        name: name.to_owned(),
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

    Ok(code.new_expr(ExprType::Block { lines: statements }))
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

        Float(num) => code.new_expr(Const {
            val: Constant::Float(num.parse().unwrap()),
        }),

        Char(c) => code.new_expr(Const {
            val: Constant::Char(c),
        }),

        // Bracketed expressions.
        LBrack => {
            let expr = parse_expr(code, 0)?;
            code.expect_else(RBrack, UnclosedBrack)?;
            expr
        }

        // Block expressions.
        LSquirl | Indent => parse_block(code)?,

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

        _ => {
            return Err(ParseError {
                err: BadExpr,
                span: code.last_idx(),
            });
        }
    };

    loop {
        // Field access. Duplicates parse_atom, because that doesn't work on structs. Refactor?
        if matches!(code.peek(), Some(Period)) {
            code.next();
            let field = code.expect_ident_else(InvalidField)?;
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

fn parse_match(code: &mut Cursor) -> Result<Expr, ParseError> {
    code.expect_ident()?;
    let expr = Box::new(parse_expr(code, 0)?);
    code.expect(Colon)?;
    code.expect(Newline)?;
    code.expect(Indent)?;

    let mut grds = Vec::new();
    loop {
        if Some(Guard) != code.peek() {
            break;
        }
        code.next();
        let patt = parse_pattern(code)?;
        code.expect(Arrow)?;

        // We make the expr a block because it has its own scope.
        let then_expr = parse_expr(code, 0)?;
        let block_scope = vec![code.new_node(NodeType::Statement { expr: then_expr })];
        let expr = code.new_expr(Block { lines: block_scope });

        grds.push(code.new_node(NodeType::Guard { patt, expr }));
        // Comma? Does expr consume last token?
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
                let payload = code.expect_ident_else(BadPattern)?;
                code.expect_else(RBrack, BadPattern)?;
                Pattern::Variant { name: var, payload }
            }

            // Variable
            _ => Pattern::Var { name: var },
        },

        // Values
        Some(Num(num)) => Pattern::Val {
            val: Constant::Num(num.parse().unwrap()),
        },
        Some(Float(num)) => Pattern::Val {
            val: Constant::Float(num.parse().unwrap()),
        },
        Some(Char(c)) => Pattern::Val {
            val: Constant::Char(c),
        },

        // All
        Some(Underscore) => Pattern::All,

        _ => {
            return Err(ParseError {
                err: ParseErrorType::BadPattern,
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

    use crate::cli::utils::load_file;

    code.expect_ident()?;
    let module_name = parse_path(code)?;
    let mod_ast = match load_file(module_name.vec()) {
        Ok(file_str) => {
            let tokens = tokenize_code(&file_str);
            let fname = module_name.base(); // base() and not fname().
            parse_file(tokens, &fname)?
        }
        Err(_) => {
            return Err(ParseError {
                err: ParseErrorType::ModuleNotFound,
                span: code.last_idx(),
            });
        }
    };
    let root = Box::new(mod_ast);
    let name = if matches!(code.peek(), Some(At)) {
        code.next();
        code.expect_ident_else(ParseErrorType::ImportNoAlias)?
    } else {
        module_name.base()
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
    code.expect_else(Colon, EnumNoBlock)?;
    code.expect_else(Newline, EnumNoBlock)?;
    code.expect_else(Indent, EnumNoBlock)?;
    let mut variants = Vec::new();
    loop {
        let name = code.expect_ident()?;

        // Check for any duplicate variants.
        if variants.iter().any(|v: &EnumVariant| v.name == name) {
            return Err(ParseError {
                err: EnumDuplicateVariant,
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

fn parse_enum(code: &mut Cursor) -> Result<Option<Box<Expr>>, ParseError> {
    if code.peek() != Some(LSquare) {
        return Ok(None);
    }
    code.expect_else(LSquare, EnumBadSyntax)?;
    let payload = Box::new(parse_expr(code, 0)?);
    code.expect_else(RSquare, EnumBadSyntax)?;
    Ok(Some(payload))
}

fn parse_struct_dec(code: &mut Cursor) -> Result<Node, ParseError> {
    code.expect_ident()?;
    let name = code.expect_ident()?;
    code.expect_else(Colon, StructNoBlock)?;
    code.expect_else(Newline, StructNoBlock)?;
    code.expect_else(Indent, StructNoBlock)?;

    let mut fields = Vec::new();
    loop {
        let field = code.expect_ident_else(StructBadSyntax)?;

        // Check for any duplicate fields.
        // Maybe use HashMap instead of Vec?
        if fields.iter().any(|f: &Node| {
            let VarDec { name, .. } = f.node.clone() else {
                unreachable!()
            };
            name == field
        }) {
            return Err(ParseError {
                err: StructDuplicateField,
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
    loop {
        let field = code.expect_ident_else(StructBadSyntax)?;

        let path = Path::from_str(&field);
        let first = Box::new(code.new_expr(Var { path }));

        code.expect_else(Op(Operator::Assign), StructNoFieldInit)?;

        let second = Box::new(parse_expr(code, 0)?);

        fields.push(code.new_expr(BinOp {
            first,
            op: Operator::Assign,
            second,
        }));

        match code.next() {
            Some(RSquare) => break,

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

fn parse_path(code: &mut Cursor) -> Result<Path, ParseError> {
    let mut path = Path::new();
    loop {
        path.push(code.expect_ident_else(BadPath)?);
        if matches!(code.peek(), Some(Separator)) {
            code.next();
        } else {
            break;
        }
    }
    Ok(path)
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

    let then = Box::new(parse_block(code)?);

    let else_block = if let Some(Ident(tok)) = code.peek() {
        match tok.as_str() {
            "else" => {
                code.next();
                code.expect_else(Colon, IfNoBlock)?;
                code.expect_else(Newline, IfNoBlock)?;
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
