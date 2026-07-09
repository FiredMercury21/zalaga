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
    Assign,
    
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

fn conv_code_ws(code: &str) -> String {
    let mut output = String::new();
    for line in code.lines() {
        let idx = line
            .find( |c: char| !c.is_whitespace() )
            .unwrap_or( line.len() );
        let (indent, post) = line.split_at(idx);
        output.push_str(&(indent.replace("    ", "\t") + post + "\n"))
    }
    output
}

/*---Lexer---*/

pub fn tokenize_code(code: &str) -> Vec<Token> {
    use TokType::*;
    use Operator::*;

    let cleaned = conv_code_ws(code);
    let look = &mut cleaned.chars().peekable();
    let mut output = Vec::new();
    let mut stream = Vec::new();
    //let mut multi_str = String::new();
    let mut line_idx = 1;
    let mut idx = 1;
    let mut prev_idx = 1;

    while let Some(c) = look.next() {
        prev_idx = idx;
        stream.push( Token { tok_type:
            // Handle multi-line strings.
            // Don't think we need it anymore?
            /*
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
            } else { */
                match c {

                    // Early end loop
                    ' '  => continue,  

                    // Stop chars.
                    // Surely there's a more elegant way?
                    '('  => { idx += 1; LBrack },
                    ')'  => { idx += 1; RBrack },
                    '{'  => { idx += 1; LSquirl },
                    '}'  => { idx += 1; RSquirl }, 
                    '['  => { idx += 1; LSquare }, 
                    ']'  => { idx += 1; RSquare }, 
                    ':'  => { idx += 1; Colon }, 
                    ';'  => { idx += 1; SColon },                     
                    ','  => { idx += 1; Comma },
                    '.'  => { idx += 1; Period }, 

                    '#'  => { idx += 1; Op(Deref) }
                    '^'  => { idx += 1; Op(Exp) }
                    '%'  => { idx += 1; Op(Mod) }

                    '\t' => { idx += 4; Indent },
            
                    '\n' => { idx = 1; line_idx += 1; Newline },

                    // Could make a macro for these
                    '-' => match look.peek() {
                        Some('>') => { look.next(); idx += 2; Arrow },
                        Some('-') => { look.next(); idx += 2; Op(Dec) },
                        _         => {              idx += 1; Op(Sub) }
                    },

                    '&' => match look.peek() {
                        Some('&') => { look.next(); idx += 2; Op(And) },
                        _         => {              idx += 1; Op(Ref) }
                    },

                    '|' => match look.peek() {
                        Some('|') => { look.next(); idx += 2; Op(Or) },
                        _         => {              idx += 1; Guard }
                    },

                    '+' => match look.peek() {
                        Some('+') => { look.next(); idx += 2; Op(Inc) },
                        _         => {              idx += 1; Op(Add) }
                    },

                    '>' => match look.peek() {
                        Some('=') => { look.next(); idx += 2; Op(GorET) },
                        _         => {              idx += 1; Op(GT) }
                    },

                    '<' => match look.peek() {
                        Some('=') => { look.next(); idx += 2; Op(LorET) },
                        _         => {              idx += 1; Op(LT) }
                    },

                    '!' => match look.peek() {
                        Some('-') => { look.next(); idx += 2; Op(NotET) },
                        _         => {              idx += 1; Op(Neg) }
                    },

                    '=' => match look.peek() {
                        Some('=') => { look.next(); idx += 2; Op(ET) },
                        _         => {              idx += 1; Assign }
                    },

                    '/' => match look.peek() {
                        Some('/') => { 
                            look.next(); 
                            look.peek_while::<_, String>(|c: &char| *c != '\n');
                            continue;
                        },
                        _ => { idx += 1; Op(Div) }
                    },

                    // String
                    // We'll add it later...
                    /*
                    '"' => {
                        let body = look.peek_while(|c: &char| *c != '"');
                        // This is a remnant from when this was line-by-line?
                        /*
                        match look.next() {  
                            Some(_) => Str(body),
                            None => {
                                multi_str.push_str(&body);            
                                continue;
                            }
                        }
                        */
                        Str(body)
                    },
                    */

                    // Number
                    c if c.is_ascii_digit() => {
                        let dig = c.to_string() + &(look.peek_while::<_, String>(|c: &char| c.is_ascii_digit()));
                        let post = match look.peek() {
                            Some('.') => {
                                look.next();
                                look.peek_while::<_, String>(|c: &char| c.is_ascii_digit())
                            },
                            _ => "".to_string()
                        };
                        let num =  dig + &post;
                        idx += num.len();
                        Num(num)
                    },

                    // Identifier, or...
                    c if c.is_ascii() => {
                        let post = look.peek_while::<_, String>(|c: &char| c.is_ascii_alphanumeric() || *c == '_');
                        let ident = c.to_string() + &post;
                        idx += ident.len();
                        Ident(ident)
                    },

                    // Else
                    _ => Illegal(c)

                },
        //}, 
        
        index: Span { line: line_idx.clone(), idx: prev_idx } } )
    }
    /* 
        The stupid indents. They preface each line. Need to cut them down
        to just single Indents and Dedents when needed.
        TODO: Make first line's indents work appropriately.
    */
    let mut prev_indent = 0;      
    let mut current_indent = 0;
    let mut stream = stream.iter().peekable();
    while let Some(tok) = stream.next() {
        output.push(tok.clone());
        if let Newline = tok.tok_type {
            current_indent = 0;

            // Every new line, observe indent count
            while let Some(tok) = stream.next() {
                match tok.tok_type {
                    // If more count, indent
                    Indent => {
                        current_indent += 1;
                        if current_indent > prev_indent {
                            output.push(Token { tok_type: Indent, index: tok.index.clone() });
                        }
                    },
                    
                    // If a blank line, match indent count.
                    Newline => {
                        for _ in 0..(current_indent - prev_indent) { output.pop(); };
                        current_indent = 0;
                    },

                    // Once you get to non-indent, break from indent count
                    _ => { 
                        output.push(tok.clone()); 
                        break 
                    }
                }
            }

            // If less count, dedent
            if current_indent < prev_indent {
                for _ in 0..(prev_indent - current_indent) {
                    output.push(Token { tok_type: Dedent, index: Span { line: tok.index.line, idx: 0 } });
                }
            }
            prev_indent = current_indent;
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