use std::iter::Peekable;

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
    Period,
    Arrow,
    Indent,
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

fn is_key(c: &char) -> bool {
    matches!(c, ' ' | '\t' | '.' | '\n' | ',' | '(' | '|' | ')' | '{' | '}' | ':' | '"')
}

fn conv_line_ws(line: &str) -> String {
    let idx = line.find( |c: char| !c.is_whitespace() )
                  .unwrap_or( line.len() );
    let (indent, post) = line.split_at(idx);
    //indent.replace("    ", "\t") + post
    indent.replace(" ", "\t") + post  // Single-line indentation
}

fn peek_while<I, F>(code: &mut Peekable<I>, pattern: F) -> String
where 
    I: Iterator<Item = (usize, char)>,
    F: Fn(&char) -> bool
{
    let mut output = String::new();
    while let Some((_, c)) = code.peek() {
        if pattern(c) {
            // We know code.next() works because we peeked. Ignore index.
            output.push(code.next().unwrap().1);
        } else {
            break;
        }
    }
    output
}


// Don't use this yet. conv_line_ws instead.
/*
fn check_line_ws((_, line): &(usize, &str)) -> Option<usize> {
    // We have to use char_indices for peek_while to work properly.
    let mut look = line.char_indices().peekable();
    let pre = peek_while(&mut look, |c: &char| !c.is_whitespace());

    if let Some(x) = pre.find('\t') {
        if let Some(y) = pre.find(' ') {
            // Return index of non-conforming whitespace.
            return Some( if x > y { x } else { y } )
        }
    }
    None
}
*/

pub fn tokenize_code(code: &str) -> Vec<Token> {
    let cleaned = conv_line_ws(code);
    // char_indices similar to chars().enumerate(). But UTF-8? Idk.
    let mut look = cleaned.char_indices().peekable();  
    let mut output = Vec::new();
    let mut multi_str = String::new();
    let mut line_idx = 0;
    use TokType::*;

    while let Some((idx, c)) = look.next() {
        output.push( Token { tok_type:
            // Handle multi-line strings.
            if !multi_str.is_empty() {
                if c == '"' {
                    Str(std::mem::take(&mut multi_str))
                } else {
                    // If end of line, continue multi-line string.
                    let body = peek_while(&mut look, |c: &char| *c != '"');
                    match look.next() {
                        Some(_) => {
                            multi_str.push(c);
                            multi_str.push_str(&body);
                            Str(std::mem::take(&mut multi_str))
                        }
                        None => {
                            multi_str.push(c);
                            multi_str.push_str(&body);
                            continue;
                        }
                    }
                }
            } else {
                match c {

                    // Early end loop
                    ' '  => continue,  

                    // Stop chars
                    '('  => LBrack,
                    ')'  => RBrack,
                    '{'  => LSquirl,
                    '}'  => RSquirl, 
                    ':'  => Colon, 
                    ','  => Comma,
                    '.'  => Period, 
                    '|'  => Guard,
                    '\t' => Indent,
                
                    '\n' => { line_idx += 1; Newline },

                    // String
                    '"' => {
                        let body = peek_while(&mut look, |c: &char| *c != '"');
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
                        let num = c.to_string() + &peek_while(&mut look, |c: &char| c.is_ascii_digit());
                        Num(num)
                    },

                    // Identifier
                    c if c.is_ascii() => {
                        let ident = c.to_string() + &peek_while(&mut look, |c: &char| c.is_ascii() && !is_key(c));
                        match ident.as_str() {
                            "->" => Arrow,
                            _    => Ident(ident)
                        }
                    },

                    // Else
                    _ => Illegal(c)

                }
        }, 
        
        index: Span { line: line_idx.clone(), idx: idx } } )
    }

    output
}

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