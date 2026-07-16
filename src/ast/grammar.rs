use crate::ast::ast_types::*;
use crate::ast::lexer::Span;
use std::collections::HashMap;

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
    //String,
    Void,
    Never,
    Ref(Box<TypeType>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeType {
    Prim(Prim),
    //Array(Box<Type>), // Pointers bby!
    Struct(Struct),
    Enum(Enum),
    Fn {
        args: Vec<Type>,
        ret: Box<Type>,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    ty: TypeType,
    size: u32, // Bytes. Problems if more than a couple GB?
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<(String, Type, u32)>, // (name, type, byte offset)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<(String, Type, u32)>,
    pub current: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub expected: Type,
    pub actual: Type,
    pub location: Span,
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct ScopeError {
//     pub kind: ScopeErrorKind,
//     pub location: Span,
// }

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeError {
    UndefinedType { name: String },
    UndefinedVar { name: String },
    UndefinedFn { name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub parent: Option<usize>, // Vector-tree index.
    pub vars: HashMap<String, Type>,
    pub types: HashMap<String, Type>,
    pub functions: HashMap<String, Type>,
    pub node: Id,  // Unneeded?
}

// Flat vector-tree. Why this now, instead of the box approach earlier?
// Because I was dumb earlier, fuck you.
// Man, would be SO much easier if I could just access nodes as usize!!
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeTable {
    pub scopes: Vec<Scope>,           // indexed by scope id
    pub node_scope: HashMap<Id, usize>,  // node id -> scope id
}

// Size of primitives in bytes.
fn size_prim(prim: &Prim) -> Type {
    use Prim::*;
    Type {
        ty: TypeType::Prim(prim.clone()),
        // Maybe use a HashMap?
        size: match prim {
            Char => 1,
            Int16 => 2,
            Int32 => 4,
            Int64 => 8,
            Float16 => 2,
            Float32 => 4,
            Float64 => 8,
            Bool => 1,
            //String => 8,
            Void => 0,
            Never => 0,
            Ref(_) => 8, // x64 bby!!
        },
    }
}

fn size_type(ty: &TypeType) -> Type {
    use TypeType::*;
    
    Type {
        ty: ty.clone(),
        size: match ty {
            Prim(prim) => size_prim(prim).size,
            Struct(self::Struct { fields, .. }) => fields
                .iter()
                .map(|(_, Type { size, .. }, _)| size) // Should we recurse or trust upstream?
                .sum(),
            Enum(self::Enum { variants, .. }) => variants
                .iter()
                .map(|(_, Type { size, .. }, _)| size)
                .max()
                .unwrap_or(&0) // C-style enum. Currently only via annotating as void. Add Option?
                + 1, // One byte for the variant index. Needed?
            Fn { .. } => 8, // Function pointers???
        }
    }
}

fn populate_scope(root: &Node) -> Result<ScopeTable, ScopeError> {
    let mut table = ScopeTable {
        scopes: Vec::new(),
        node_scope: HashMap::new(),
    };
    scope_node(&mut table, root, None)?;
    Ok(table)
}

fn new_scope(table: &mut ScopeTable, parent: Option<usize>, id: Id) -> usize {
    table.scopes.push(Scope {
        parent,
        vars:      HashMap::<String, self::Type>::new(),
        types:     HashMap::<String, self::Type>::new(),
        functions: HashMap::<String, self::Type>::new(),
        node: id,
    });
    let idx = table.scopes.len() - 1;
    table.node_scope.insert(id, idx);
    idx
}

fn scope_node(table: &mut ScopeTable, node: &Node, parent: Option<usize>) -> Result<(), ScopeError> {
    use NodeType::*;
    use ScopeError::*;
    
    
    match &node.node {
        Module { scope, .. } => {
            let idx = new_scope(table, parent, node.id);
            for node in scope {
                // Populates parent scope too. See down.
                scope_node(table, &node, Some(idx))?;
            }
        }

        FnDec { args, body, ret_type, name } => {
            // New scope for function body.
            let idx = new_scope(table, parent, body.id);
            table.node_scope.insert(body.id, idx);

            // Add each arg to current scope.
            let mut arg_types = Vec::new();
            for arg in args {
                // Each is a var dec.
                let NodeType::VarDec { name, var_type, .. } = &arg.node else { 
                    unreachable!()
                };
                // How to get non-sequential declarations?
                let arg_type = match node_to_type(var_type, parent.unwrap(), table) {
                    Some(ty) => ty,
                    None => return Err(UndefinedType { name: node_type_to_str(var_type) }),
                };
                // Add arg to scope.
                table.scopes[idx].vars.insert(name.clone(), arg_type.clone());
                arg_types.push(arg_type);
            }
            
            // Add function to parent scope.
            let fn_type = self::Type { 
                ty: TypeType::Fn { 
                    args: arg_types,
                    ret: match node_to_type(&ret_type, idx, table) {
                        Some(ty) => Box::new(ty),
                        None => return Err(UndefinedType { name: node_type_to_str(ret_type) }),
                    },
                },
                size: 0,
            };
            table.scopes[parent.unwrap()].functions.insert(name.clone(), fn_type);
            
            // Check scopes of body.
            scope_expr(table, &body, idx)?;
        }
        
        VarDec { name, expr, var_type } => {
            let ty = node_to_type(var_type, parent.unwrap(), table);
            if let Some(ty) = ty {
                table.scopes[parent.unwrap()].vars.insert(name.clone(), ty);
            } else {
                return Err(UndefinedType { name: node_type_to_str(var_type) });
            }
            if let Some(expr) = expr {
                scope_expr(table, expr, parent.unwrap())?;
            }
        }
        
        StructDec { name, fields } => {
            let mut fields_vec: Vec<(String, self::Type, u32)> = Vec::new();
            let mut offset = 0;
            for field in fields {
                // Each is a var dec.
                let NodeType::VarDec { name, var_type, .. } = &field.node else { 
                    unreachable!()
                };

                // Check type.
                let field_type = match node_to_type(&var_type, parent.unwrap(), table) {
                    Some(ty) => ty,
                    None => return Err(UndefinedType { name: node_type_to_str(&var_type) }),
                };
                
                fields_vec.push((name.clone(), field_type.clone(), offset));
                offset += field_type.size; // Non-C-ABI compat? Change.
            }
            let ty = TypeType::Struct(Struct { name: name.clone(), fields: fields_vec });
            table.scopes[parent.unwrap()].types.insert(
                name.clone(), 
                self::Type { 
                    ty, 
                    size: offset,
                }
            );
        }
        
        EnumDec { name, variants } => {
            for variant in variants {
                
            }
        }

        Statement { expr } => {
            scope_expr(table, &expr, parent.unwrap())?;
        }
        
        VarAsn { name, val } => {
            if matches!(find_in_scope(name, table, parent.unwrap(), ScopeType::Vars), None) {
                return Err(UndefinedVar { name: name.clone() })
            }
            scope_expr(table, val, parent.unwrap())?;
        }
        
        Guard { pred, expr } => {
            scope_expr(table, pred, parent.unwrap())?;
            scope_expr(table, expr, parent.unwrap())?;
        }

        For { init, pred, then, block } => {
            let idx = new_scope(table, parent, block.id);

            // Add var to for block scope.
            let VarDec { name, expr, var_type } = &init.node else {
                unreachable!()
            };
            let ty = node_to_type(var_type, idx, table).unwrap();
            table.scopes[idx].vars.insert(name.clone(), ty);
            if let Some(expr) = expr {
                scope_expr(table, expr, parent.unwrap())?;
            }

            // Check scopes underneath.
            scope_expr(table, pred, idx)?;
            scope_expr(table, then, idx)?;
            scope_expr(table, block, idx)?;
        }
        
        While { pred, block } => {
            let idx = new_scope(table, parent, block.id);
            scope_expr(table, pred, idx)?;
            scope_expr(table, block, idx)?;
        }
        
        Return { val } => {
            scope_expr(table, val, parent.unwrap())?;
        }

        // TODO:
        Use { name } => {
            //scope_expr(table, name, Some(parent.unwrap()))?;
        }
        
        Type { .. } => {
            unreachable!() // I think?
        }
        
        Break => {},
        Continue => {},
    };
    Ok(())
}

fn node_type_to_str(node: &Node) -> String {
    let mut current = match node.node.clone() {
        NodeType::Type { name } => name,
        _ => unreachable!(),
    };
    loop {
        match current {
            TypeNode::Base(name) => {
                return name;
            },
            TypeNode::Ref(inner) => {
                current = *inner;
            },
            _ => unreachable!(),
        }
    }
}

fn node_to_type(node: &Node, idx: usize, table: &ScopeTable) -> Option<Type> {
    let NodeType::Type { name } = &node.node else {
        unreachable!()
    };
    let mut current = name;
    let mut base: Type;
    let mut ref_n = 0;
    
    loop {
        match &current {
            TypeNode::Base(name) => { 
                if let Some(idx) = find_in_scope(name, table, idx, ScopeType::Types) {
                    base = table.scopes[idx].types[name].clone();
                    break;
                } else {
                    return None
                }
            }
            TypeNode::Ref(inner) => {
                ref_n += 1;
                current = inner;
            }
        }
    }
    for _ in 0..ref_n {
        base = Type {
            ty: TypeType::Prim(Prim::Ref(Box::new(base.ty))),
            size: 8,
        }
    }
    Some(base)
}

enum ScopeType {
    Vars,
    Types,
    Functions,
}

fn find_in_scope(name: &str, table: &ScopeTable, current: usize, ty: ScopeType) -> Option<usize> {
    let mut idx = current;
    loop {
        if let Some(_) = match ty {
            ScopeType::Vars => &table.scopes[idx].vars,
            ScopeType::Types => &table.scopes[idx].types,
            ScopeType::Functions => &table.scopes[idx].functions,
        }.get(name) {
            return Some(idx)
        }
        if let Some(parent) = &table.scopes[idx].parent {
            idx = *parent;
        } else {
            break
        }
    }
    None
}

fn scope_expr(table: &mut ScopeTable, expr: &Expr, parent: usize) -> Result<(), ScopeError> {
    use ScopeError::*;
    use ExprType::*;
    
    match &expr.expr {
        Var { name } => {
            match find_in_scope(name, table, parent, ScopeType::Vars) {
                Some(_) => {}
                None => return Err(UndefinedVar { name: name.clone() }),
            }
        }
        Match { expr, grds } => {
            scope_expr(table, expr, parent)?;
            for grd in grds {
                scope_node(table, grd, Some(parent))?;
            }
        }
        If { pred, then, else_block } => {
            scope_expr(table, pred, parent)?;
            scope_expr(table, then, parent)?;
            if let Some(else_block) = else_block {
                scope_expr(table, else_block, parent)?;
            }
        }
        Block { scope } => {
            // The only expression that has its own scope!
            // Use it in others if you want...
            let idx = new_scope(table, Some(parent), expr.id);
            for node in scope {
                scope_node(table, node, Some(idx))?;
            }
        }
        FnCall { name, args } => {
            for arg in args {
                scope_expr(table, arg, parent)?;
            }
            if matches!(find_in_scope(name, table, parent, ScopeType::Functions), None) {
                return Err(UndefinedFn { name: name.clone() });
            }
        }
        Const { .. } => {}
        Field { base, .. } => {
            scope_expr(table, base, parent)?;
            // How to check? eg- *(arr + 1).field = ...
            // 
            // if matches!(find_in_scope(name, table, parent, ScopeType::Functions), None) {
            //     return Err(UndefinedVar { name: name.clone() });
            // }
        }
        Struct { name, fields } => {
            for field in fields {
                scope_node(table, field, Some(parent))?;
            }
            if matches!(find_in_scope(name, table, parent, ScopeType::Types), None) {
                return Err(UndefinedType { name: name.clone() });
            }
        }
        Enum { name, variant, .. } => {
            /*
             * Don't know yet. Wanna make tagged unions.
             */
        }
        BinOp { first, second, .. } => {
            scope_expr(table, first, parent)?;
            scope_expr(table, second, parent)?;
        }
        UnOp { expr, .. } => {
            scope_expr(table, expr, parent)?;
        }
    }
    Ok(())
}

fn synth(expr: &Expr, scope: &Scope) -> Result<Type, TypeError> {
    use ExprType::*;
    match &expr.expr {
        
    }
}

fn check_types(root: &Node) -> Result<Vec<(String, Type, Scope)>, TypeError> {
    use NodeType::*;
    let Node {
        node: NodeType::Module { scope, .. },
        ..
    } = root
    else {
        panic!("Expected a module, got {:?}", root) // Badddd.
    };

    let scope = scope.iter();
    while let Some(Node { node, .. }) = scope.next() {
        match node {
            FnDec => 
        }
    }
}

// String to primitive type
fn str_to_prim<'a>(input_str: &'a str) -> Option<Prim> {
    use Prim::*;

    Some(match input_str {
        "char" => Char,
        "short" => Int16,
        "int" => Int32,
        "long" => Int64,
        "half" => Float16,
        "float" => Float32,
        "double" => Float64,
        "bool" => Bool,
        //"str" => String,
        "void" => Void,
        "()" => Never, // When is this used?

        _ => return None,
    })
}
