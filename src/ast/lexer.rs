use super::ast_types::*;

use crate::utils::PeekExt;

/*---Types---*/

#[derive(Debug, Clone, PartialEq, Default)]
struct FilePos(usize);

impl FilePos {
    fn span(&mut self, interval: usize) -> Span {
        let span = Span {
            start: self.0,
            end: self.0 + interval,
        };
        self.advance(interval);
        span
    }
    fn advance(&mut self, interval: usize) {
        self.0 += interval
    }
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
        output.push_str(&(indent.replace("    ", "\t") + post + "\n"));
    }
    output
}

// Based off an experimental method from the std library.
fn split_once(arr: &[Token], pred: impl FnMut(&Token) -> bool) -> (&[Token], &[Token]) {
    match arr.iter().position(pred) {
        Some(i) => (&arr[..i], &arr[i..]),
        None => (&[], arr),
    }
}

/*---Lexer---*/

pub fn tokenize_code(code: &str) -> Vec<Token> {
    use Operator::*;
    use TokType::*;

    if code.is_empty() {
        return Vec::new(); // Errors downstream?
    }

    let cleaned = conv_code_ws(code);
    let look = &mut cleaned.chars().peekable();
    let mut toks = Vec::new();
    let mut interval = 0;
    let mut cursor = FilePos::default();

    while let Some(c) = look.next() {
        toks.push(Token {
            tok_type: match c {
                ' ' => {
                    cursor.advance(1);
                    continue;
                }

                // Stop chars.
                // Surely there's a more elegant way?
                '(' => {
                    interval = 1;
                    LBrack
                }
                ')' => {
                    interval = 1;
                    RBrack
                }
                '{' => {
                    interval = 1;
                    LSquirl
                }
                '}' => {
                    interval = 1;
                    RSquirl
                }
                '[' => {
                    interval = 1;
                    LSquare
                }
                ']' => {
                    interval = 1;
                    RSquare
                }

                ';' => {
                    interval = 1;
                    SColon
                }
                ',' => {
                    interval = 1;
                    Comma
                }
                '.' => {
                    interval = 1;
                    Period
                }

                '_' => {
                    interval = 1;
                    Underscore
                }

                '@' => {
                    interval = 1;
                    At
                }

                '#' => {
                    interval = 1;
                    Op(Deref)
                }

                '^' => {
                    interval = 1;
                    Op(Exp)
                }

                '*' => {
                    interval = 1;
                    Op(Mul)
                }

                '%' => {
                    interval = 1;
                    Op(Mod)
                }

                '\t' => {
                    // ERROR: Not right byte offset...
                    // For spaces are converted to tabs, but tabs can already exist.
                    interval = 4;
                    Indent
                }

                '\n' => {
                    interval = 1; // One or zero?
                    Newline
                }

                // Could make a macro for these
                // ERROR: Negative numbers.
                '-' => match look.peek() {
                    Some('>') => {
                        look.next();
                        interval = 2;
                        Arrow
                    }
                    Some('-') => {
                        look.next();
                        interval = 2;
                        Op(Dec)
                    }
                    _ => {
                        interval = 1;
                        Op(Sub)
                    }
                },

                '&' => match look.peek() {
                    Some('&') => {
                        look.next();
                        interval = 2;
                        Op(And)
                    }
                    _ => {
                        interval = 1;
                        Op(Ref)
                    }
                },

                '|' => match look.peek() {
                    Some('|') => {
                        look.next();
                        interval = 2;
                        Op(Or)
                    }
                    _ => {
                        interval = 1;
                        Guard
                    }
                },

                ':' => match look.peek() {
                    Some(':') => {
                        look.next();
                        interval = 2;
                        Separator
                    }
                    _ => {
                        interval = 1;
                        Colon
                    }
                },

                '+' => match look.peek() {
                    Some('+') => {
                        look.next();
                        interval = 2;
                        Op(Inc)
                    }
                    _ => {
                        interval = 1;
                        Op(Add)
                    }
                },

                '>' => match look.peek() {
                    Some('=') => {
                        look.next();
                        interval = 2;
                        Op(GorET)
                    }
                    _ => {
                        interval = 1;
                        Op(GT)
                    }
                },

                '<' => match look.peek() {
                    Some('=') => {
                        look.next();
                        interval = 2;
                        Op(LorET)
                    }
                    _ => {
                        interval = 1;
                        Op(LT)
                    }
                },

                '!' => match look.peek() {
                    Some('=') => {
                        look.next();
                        interval = 2;
                        Op(NotET)
                    }
                    _ => {
                        interval = 1;
                        Op(Neg)
                    }
                },

                '=' => match look.peek() {
                    Some('=') => {
                        look.next();
                        interval = 2;
                        Op(ET)
                    }
                    _ => {
                        interval = 1;
                        Op(Assign)
                    }
                },

                '\'' => match look.next() {
                    Some(x) => match look.peek() {
                        Some('\'') => {
                            look.next();
                            interval = 3;
                            Char(x)
                        }
                        _ => {
                            interval = 2;
                            Illegal(c) // x is lost!! Bad. But only in bad syntax.
                        }
                    },
                    _ => {
                        interval = 2;
                        Illegal(c)
                    }
                },

                '/' => match look.peek() {
                    // Comments
                    Some('/') => {
                        look.next();
                        cursor.advance(2);
                        let comment = look.peek_while::<_, String>(|c: &char| *c != '\n');
                        cursor.advance(comment.len());
                        continue;
                    }
                    _ => {
                        interval = 1;
                        Op(Div)
                    }
                },

                // Number
                c if c.is_ascii_digit() => {
                    let dig = c.to_string()
                        + &(look.peek_while::<_, String>(|c: &char| c.is_ascii_digit()));
                    if Some(&'.') == look.peek() {
                        look.next();
                        let post = look.peek_while::<_, String>(|c: &char| c.is_ascii_digit());
                        let num = dig + "." + &post;
                        interval = num.len();
                        Float(num)
                    } else {
                        interval = dig.len();
                        Num(dig)
                    }
                }

                // Identifier, or...
                c if c.is_ascii() => {
                    let post = look
                        .peek_while::<_, String>(|c: &char| c.is_ascii_alphanumeric() || *c == '_');
                    let ident = c.to_string() + &post;
                    interval = ident.len();
                    Ident(ident)
                }

                // Else
                _ => {
                    interval = 1;
                    Illegal(c)
                }
            },

            index: cursor.span(interval),
        });
    }

    // The stupid indents. They preface each line. Need to cut them down
    // to just single Indents and Dedents when needed.

    // Nasty method chaining, but the imperative version was way worse.
    // I don't like how many type annotations are needed. Any way to elide?
    let mut output: Vec<Token> = toks
        // Split by Newline
        .split(|tok| tok.tok_type == Newline)
        // Split lines into (indents, post)
        .map(|line| split_once(line, |tok| tok.tok_type != Indent))
        // Remove all blank lines
        .filter(|&(_, post): &(_, &[Token])| !post.is_empty())
        // Add an empty line at end to make last indents work.
        .chain(std::iter::once((&[] as &[Token], &[] as &[Token])))
        // Turn into vec for windows() to pair stuff
        .collect::<Vec<(&[Token], &[Token])>>()
        // Pair up lines for indent delta calc
        .windows(2)
        // Check indent deltas and generate proper line.
        .flat_map(|w: &[(&[Token], &[Token])]| {
            // w[1] = next line, w[0] = this line, both are (indents, post).
            let indent_delta = (w[1].0.len() as isize) - (w[0].0.len() as isize);
            let mut indents = Vec::new();
            let next_line_idx = w[1]
                .1
                .get(0)
                .map(|tok| tok.index.clone())
                .unwrap_or(cursor.span(0));
            for i in 0..indent_delta.unsigned_abs() {
                // To make spans work, we copy from actual line.
                indents.push(if indent_delta > 0 {
                    Token {
                        tok_type: Indent,
                        index: w[1].0[i].index.clone(),
                    }
                } else {
                    Token {
                        tok_type: Dedent,
                        index: next_line_idx.clone(),
                    }
                });
            }
            // Add Newline temporary.
            let newline = Token {
                tok_type: Newline,
                index: next_line_idx,
            };
            // Return the 'post' of this line with indents appended.
            w[0].1
                .iter()
                .cloned()
                .chain(std::iter::once(newline))
                .chain(indents.iter().cloned())
                .collect::<Vec<Token>>()
        })
        .collect();

    // Add EOF.
    output.push(Token {
        tok_type: Eof,
        index: output
            .last()
            .map(|tok| tok.index.clone())
            .unwrap_or(cursor.span(0)),
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
