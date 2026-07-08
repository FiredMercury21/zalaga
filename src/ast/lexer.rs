use crate::utils::PeekExt;

/*---Types---*/

#[derive(Debug, Clone, PartialEq)]
pub enum TokType {
    Eof,
    Guard,
    Comma,
    LBrack,
    RBrack,
    LSquirl,
    RSquirl,
    Colon,
    SColon,
    Period,
    Arrow,
    Assign,
    Indent,
    Dedent,
    Newline,
    Illegal(char),
    Num(String),
    Str(String),
    Ident(String)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub idx: usize
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok_type: TokType,
    pub index: Span
}

/*---Helper functions---*/

fn is_key(c: &char) -> bool {
    matches!(c, ' ' | '\t' | '.' | '\n' | ',' | '(' | '|' | ')' | '{' | '}' | ':' | '"')
}

fn conv_line_ws(line: &str) -> String {
    let idx = line.find( |c: char| !c.is_whitespace() )
                  .unwrap_or( line.len() );
    let (indent, post) = line.split_at(idx);
    indent.replace("    ", "\t") + post 
}

/*---Lexer---*/

pub fn tokenize_code(code: &str) -> Vec<Token> {
    let cleaned = conv_line_ws(code);
    let look = &mut cleaned.chars().peekable();
    let mut output = Vec::new();
    let mut stream = Vec::new();
    let mut multi_str = String::new();
    let mut line_idx = 0;
    let mut idx = 0;
    use TokType::*;

    while let Some(c) = look.next() {
        stream.push( Token { tok_type:
            // Handle multi-line strings.
            if !multi_str.is_empty() {
                if c == '"' {
                    line_idx += 1;
                    idx += 1;
                    Str(std::mem::take(&mut multi_str))
                } else {
                    // If end of line, continue multi-line string.
                    let body: String = look.peek_while(|c: &char| *c != '"');
                    match look.next() {
                        Some(_) => {
                            multi_str.push(c);
                            multi_str.push_str(&body);
                            idx += body.len() + 1;
                            Str(std::mem::take(&mut multi_str))
                        }
                        None => {
                            multi_str.push(c);
                            multi_str.push_str(&body);                    
                            line_idx += 1;
                            idx += body.len() + 1;
                            continue;
                        }
                    }
                }
            } else {
                match c {

                    // Early end loop
                    ' '  => continue,  

                    // Stop chars.
                    // Surely there's a more elegant way?
                    '('  => { idx += 1; LBrack },
                    ')'  => { idx += 1; RBrack },
                    '{'  => { idx += 1; LSquirl },
                    '}'  => { idx += 1; RSquirl }, 
                    ':'  => { idx += 1; Colon }, 
                    ';'  => { idx += 1; SColon }, 
                    ','  => { idx += 1; Comma },
                    '.'  => { idx += 1; Period }, 

                    '\t' => { idx += 4; Indent },
                
                    '\n' => { idx = 0; line_idx += 1; Newline },

                    // String
                    '"' => {
                        let body = look.peek_while(|c: &char| *c != '"');
                        // Multi-line handled above.
                        match look.next() {  
                            Some(_) => Str(body),
                            None => {
                                multi_str.push_str(&body);            
                                continue;
                            }
                        }
                    },

                    // Number
                    c if c.is_ascii_digit() => {
                        let post = look.peek_while::<_, String>(|c: &char| c.is_ascii() && !is_key(c));
                        let num = c.to_string() + &post;
                        idx += num.len();
                        Num(num)
                    },

                    // Identifier, or...
                    c if c.is_ascii() => {
                        let post = look.peek_while::<_, String>(|c: &char| c.is_ascii() && !is_key(c));
                        let ident = c.to_string() + &post;
                        idx += ident.len();
                        // Some keywords could have multiple chars,
                        // or be prefixes (e.g. '=' and '==')
                        match ident.as_str() {
                            "->" => Arrow,
                            "="  => Assign,
                            "|"  => Guard,
                            _    => Ident(ident)
                        }
                    },

                    // Else
                    _ => Illegal(c)

                }
        }, 
        
        index: Span { line: line_idx.clone(), idx: idx } } )
    }

    /* 
        The stupid indents. They preface each line. Need to cut them down
        to just single Indents and Dedents when needed.
        TODO: Make first line's indents work appropriately.
    */
    let mut prev_indent = 0;      
    let mut stream = stream.iter().peekable();
    while let Some(&tok) = stream.peek() {
        let mut current_indent = 0;
        match tok.tok_type {
            Newline => {
                stream.next();

                while let Some(&tok) = stream.peek() {
                    match tok.tok_type {
                        // If more, indent
                        Indent => { 
                            stream.next();
                            current_indent += 1;
                            // If more, increment
                            if current_indent > prev_indent {
                                output.push(Token { tok_type: Indent, index: tok.index.clone() });
                            }
                        },
                        
                        // If a blank line
                        Newline => {
                            if let Some(Token { tok_type: Newline, .. }) = stream.peek() {
                                for _ in 0..(current_indent - prev_indent) { output.pop(); };
                                current_indent = prev_indent;
                            }
                        },

                        _ => break
                    }
                }

                // If less, dedent
                if current_indent < prev_indent {
                    for _ in 0..(prev_indent - current_indent) {
                        output.push(Token { tok_type: Dedent, index: Span { line: tok.index.line, idx: 0 } });
                    }
                }
                prev_indent = current_indent;
            },

            _ => { 
                stream.next();
                output.push(tok.clone()) 
            }
        }
    }

    output.push(Token { tok_type: Eof, index: Span { line: line_idx + 1, idx: 0 } });

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
}