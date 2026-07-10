/*
use crate::ast::tree::*;

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
        "void"   => Void, // Should we use never?

        _ => return None
    })
}
*/