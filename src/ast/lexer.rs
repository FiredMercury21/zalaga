use crate::utils::PeekExt;

/*---Types---*/

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

    // Operators
    Op(Operator),

    // Constants
    Num(String),

    // Identifiers
    Ident(String),

    Illegal(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok_type: TokType,
    pub index: Span,
}

/*---Helper functions---*/

fn conv_code_ws(code: &str) -> String {
    let mut output = String::new();
    for line in code.lines() {
        let idx = line
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(line.len());
        let (indent, post) = line.split_at(idx);
        // Maybe count whitespaces and find common denominator?
        output.push_str(&(indent.replace("    ", "\t") + post + "\n"))
    }
    output
}

// Based off an experimental method from the std library.
fn split_once(arr: &[Token], mut pred: impl FnMut(&Token) -> bool) -> (&[Token], &[Token]) {
    match arr.iter().position(|t| pred(t)) {
        Some(i) => (&arr[..i], &arr[i..]),
        None => (&[], arr),
    }
}

// Check line number of split_once output, returning None if both empty.
fn try_find_line((indents, tokens): &(&[Token], &[Token])) -> Option<usize> {
    indents.first().or(tokens.first()).map(|tok| tok.index.line)
}

/*---Lexer---*/

pub fn tokenize_code(code: &str) -> Vec<Token> {
    use Operator::*;
    use TokType::*;

    let cleaned = conv_code_ws(code);
    let look = &mut cleaned.chars().peekable();
    let mut output = Vec::new();
    let mut stream = Vec::new();
    let mut line_idx = 1;
    let mut idx = 1;
    let mut prev_idx;

    while let Some(c) = look.next() {
        prev_idx = idx;
        stream.push(Token {
            tok_type: match c {
                ' ' => continue,

                // Stop chars.
                // Surely there's a more elegant way?
                '(' => {
                    idx += 1;
                    LBrack
                }
                ')' => {
                    idx += 1;
                    RBrack
                }
                '{' => {
                    idx += 1;
                    LSquirl
                }
                '}' => {
                    idx += 1;
                    RSquirl
                }
                '[' => {
                    idx += 1;
                    LSquare
                }
                ']' => {
                    idx += 1;
                    RSquare
                }
                ':' => {
                    idx += 1;
                    Colon
                }
                ';' => {
                    idx += 1;
                    SColon
                }
                ',' => {
                    idx += 1;
                    Comma
                }
                '.' => {
                    idx += 1;
                    Period
                }

                '@' => {
                    idx += 1;
                    At
                }

                '#' => {
                    idx += 1;
                    Op(Deref)
                }

                '^' => {
                    idx += 1;
                    Op(Exp)
                }

                '*' => {
                    idx += 1;
                    Op(Mul)
                }

                '%' => {
                    idx += 1;
                    Op(Mod)
                }

                '\t' => {
                    idx += 4;
                    Indent
                }

                '\n' => {
                    idx = 1;
                    line_idx += 1;
                    Newline
                }

                // Could make a macro for these
                // ERROR: Negative numbers.
                '-' => match look.peek() {
                    Some('>') => {
                        look.next();
                        idx += 2;
                        Arrow
                    }
                    Some('-') => {
                        look.next();
                        idx += 2;
                        Op(Dec)
                    }
                    _ => {
                        idx += 1;
                        Op(Sub)
                    }
                },

                '&' => match look.peek() {
                    Some('&') => {
                        look.next();
                        idx += 2;
                        Op(And)
                    }
                    _ => {
                        idx += 1;
                        Op(Ref)
                    }
                },

                '|' => match look.peek() {
                    Some('|') => {
                        look.next();
                        idx += 2;
                        Op(Or)
                    }
                    _ => {
                        idx += 1;
                        Guard
                    }
                },

                '+' => match look.peek() {
                    Some('+') => {
                        look.next();
                        idx += 2;
                        Op(Inc)
                    }
                    _ => {
                        idx += 1;
                        Op(Add)
                    }
                },

                '>' => match look.peek() {
                    Some('=') => {
                        look.next();
                        idx += 2;
                        Op(GorET)
                    }
                    _ => {
                        idx += 1;
                        Op(GT)
                    }
                },

                '<' => match look.peek() {
                    Some('=') => {
                        look.next();
                        idx += 2;
                        Op(LorET)
                    }
                    _ => {
                        idx += 1;
                        Op(LT)
                    }
                },

                '!' => match look.peek() {
                    Some('-') => {
                        look.next();
                        idx += 2;
                        Op(NotET)
                    }
                    _ => {
                        idx += 1;
                        Op(Neg)
                    }
                },

                '=' => match look.peek() {
                    Some('=') => {
                        look.next();
                        idx += 2;
                        Op(ET)
                    }
                    _ => {
                        idx += 1;
                        Op(Assign)
                    }
                },

                '/' => match look.peek() {
                    // Comments
                    Some('/') => {
                        look.next();
                        look.peek_while::<_, String>(|c: &char| *c != '\n');
                        continue;
                    }
                    _ => {
                        idx += 1;
                        Op(Div)
                    }
                },

                // Number
                c if c.is_ascii_digit() => {
                    let dig = c.to_string()
                        + &(look.peek_while::<_, String>(|c: &char| c.is_ascii_digit()));
                    let post = match look.peek() {
                        Some('.') => {
                            look.next();
                            look.peek_while::<_, String>(|c: &char| c.is_ascii_digit())
                        }
                        _ => "".to_string(),
                    };
                    let num = dig + &post;
                    idx += num.len();
                    Num(num)
                }

                // Identifier, or...
                c if c.is_ascii() => {
                    let post = look
                        .peek_while::<_, String>(|c: &char| c.is_ascii_alphanumeric() || *c == '_');
                    let ident = c.to_string() + &post;
                    idx += ident.len();
                    Ident(ident)
                }

                // Else
                _ => Illegal(c),
            },

            index: Span {
                line: line_idx.clone(),
                idx: prev_idx,
            },
        })
    }

    // The stupid indents. They preface each line. Need to cut them down
    // to just single Indents and Dedents when needed.

    // Nasty method chaining, but the imperative way was way worse.
    let mut indent_n: Vec<(i32, bool)> = stream
        .split(|tok| tok.tok_type == TokType::Newline)
        .map(|line| {
            line.iter().fold((0, false), |acc, tok| {
                if let TokType::Indent = tok.tok_type {
                    (acc.0 + 1, acc.1)
                } else {
                    (acc.0, true)
                }
            })
        })
        .collect();

    if !indent_n.last().unwrap().1 {
        indent_n.last_mut().unwrap().0 = 0;
    }
    for i in (0..indent_n.len().saturating_sub(2)).rev() {
        if !indent_n[i].1 {
            indent_n[i].0 = indent_n[i + 1].0;
        }
    }

    // Really weird. Tuple nightmare.
    // stream[i].0 is the indent block, stream[i].1 is the rest of the line.
    let stream: Vec<(&[Token], &[Token])> = stream
        .split_inclusive(|tok| tok.tok_type == TokType::Newline)
        .map(|line| split_once(line, |tok| tok.tok_type != TokType::Indent))
        .collect();

    output.extend_from_slice(&stream[0].1);
    for i in 1..(stream.len() - 1) {
        let indent_delta = indent_n[i].0 - indent_n[i - 1].0;
        output.extend_from_slice(
            &(if indent_delta > 0 {
                // We copy from the indent block, stream[i].0.
                stream[i].0[..indent_delta as usize].to_vec()
            } else if indent_delta < 0 {
                // We add dedents to the output.
                vec![
                    Token {
                        tok_type: Dedent,
                        index: Span {
                            line: try_find_line(&stream[i]).unwrap_or(
                                try_find_line(&stream[i - 1])
                                    .unwrap_or(output.last().unwrap().index.line)
                            ),
                            idx: 0
                        } // Ewww.
                    };
                    indent_delta.abs() as usize
                ]
            } else {
                vec![]
            }),
        );
        // Copy the rest of the line.
        output.extend_from_slice(&stream[i].1);
    }

    output.push(Token {
        tok_type: Eof,
        index: Span {
            line: output.last().unwrap().index.line + 1,
            idx: 0,
        },
    });

    output
}

/*---Tests---*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_toks() {
        let code = " \t \t ident stuff 323 |||| ((\"))({},, : \n\".  tsrtsr\"tes\"t 32 >= 15";
        let tokenized = tokenize_code(code);
        println!("Tokens:\n");
        println!("{:#?}\n", tokenized);
    }

    #[test]
    fn test_tok_string() {
        let code = "\"Hello! Single string.\" \"This is a multi-line string\n, see?\"";
        let tokenized = tokenize_code(code);
        println!("Tokens:\n");
        println!("{:#?}\n", tokenized);
    }

    #[test]
    fn test_quicksort_tok() {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::open("./examples/quicksort.zg").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        println!("{:#?}", tokenize_code(&contents));
    }
}
