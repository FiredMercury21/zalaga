use crate::ast::ast_types::NodeType::Module;
use crate::ast::ast_types::*;
use crate::semantics::types::*;
use std::collections::HashMap;

// String to primitive type
fn str_to_prim(input_str: &str) -> Option<Prim> {
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
        "never" => Never,

        _ => return None,
    })
}

// Populates scope table for AST root node, checking
// scope of every node & expression along the way.
fn populate_scope(root: &Node) -> Result<ScopeTable, ScopeError> {
    // Init table and scope.
    let mut table = ScopeTable {
        scopes: Vec::new(),
        node_scope: HashMap::new(),
    };
    let global_id = table.new_scope(None, root.id); // Should be 0.
    let Module { global, .. } = &root.node else {
        unreachable!()
    };

    // Small pass for declarations.
    for node in global {
        register_dec(&mut table, node, global_id)?;
    }

    // Check scope of everything.
    for node in global {
        scope_node(&mut table, node, global_id)?;
    }

    Ok(table)
}

// Converts a NodeType::Type node to a string.
fn node_type_to_str(node: &Node) -> String {
    let NodeType::Type { name: mut current } = node.node.clone() else {
        unreachable!()
    };
    loop {
        match current {
            TypeNode::Base(path) => {
                return path.base();
            }
            TypeNode::Ref(inner) => {
                current = *inner;
            }
        }
    }
}

// Converts Node { NodeType::Type { .. } .. } to self::Type.
// REALLY gotta choose more descriptive names, damn.
fn node_to_type(node: &Node, idx: usize, table: &ScopeTable) -> Option<Type> {
    let NodeType::Type { name } = &node.node else {
        unreachable!()
    };
    let mut current = name;
    let mut base: Type;
    let mut ref_n = 0;

    loop {
        match current {
            TypeNode::Base(path) => {
                if let Some(ty) = str_to_prim(&path.base()) {
                    base = TypeType::Prim(ty).to_type();
                    break;
                }
                if let Some(ty) = table.get_type(path, idx) {
                    base = ty;
                    break;
                }
                return None;
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

// Checks scope of a node!
fn scope_node(table: &mut ScopeTable, node: &Node, current: usize) -> Result<(), ScopeError> {
    use NodeType::*;

    match &node.node {
        // Handled by other guards using register_dec() on blocks.
        FnDec { .. } | StructDec { .. } | EnumDec { .. } | VarDec { .. } => {}

        Statement { expr } => {
            scope_expr(table, expr, current)?;
        }

        Guard { patt, expr } => {
            use Pattern::*;

            match patt {
                All | Val { .. } => scope_expr(table, expr, current)?,
                Variant { payload: var, .. } | Var { name: var } => {
                    // expr is a block. We need to inject bound var or payload into it.
                    // For now, put the placeholder type of Never on the var.
                    // It shouldn't cause any issues? We check types later.
                    let idx = table.new_scope(Some(current), expr.id);
                    table.scopes[idx]
                        .vars
                        .insert(var.clone(), self::Type::never());
                    scope_expr(table, expr, idx)?;
                }
            }
        }

        // TODO:
        // Both for and while have predicates. This split control flow.
        // That conflicts whether or not a variable is initialized.
        // So we need to resolve it somehow between branches. Ughhh.
        // Prob done in flow_graph.
        For {
            init,
            pred,
            then,
            block,
        } => {
            let idx = table.new_scope(Some(current), block.id);

            // Add var to for block scope.
            register_dec(table, init, idx)?;

            // Check scopes underneath.
            scope_expr(table, pred, idx)?;
            scope_expr(table, then, idx)?;
            scope_expr(table, block, idx)?;
        }

        While { pred, block } => {
            let idx = table.new_scope(Some(current), block.id);
            scope_expr(table, pred, idx)?;
            scope_expr(table, block, idx)?;
        }

        Use { name, root } => {
            let module = populate_scope(root)?;
            table.scopes[current].modules.insert(name.clone(), module);
        }

        Type { .. } => unreachable!(),
        Module { .. } => unreachable!(),
    }
    Ok(())
}

// If given node is declaration, add it to scope. Else, ignore.
fn register_dec(table: &mut ScopeTable, root: &Node, current: usize) -> Result<(), ScopeError> {
    use NodeType::*;
    use ScopeError::*;

    match &root.node {
        FnDec {
            args,
            body,
            ret_type,
            name,
        } => {
            // Check if function already declared in current scope
            if table.scopes[current].functions.contains_key(name) {
                return Err(AlreadyDeclared { name: name.clone() });
            }

            // New scope for function body.
            let idx = table.new_scope(Some(current), body.id);

            // Add each arg to current scope.
            let mut arg_types = Vec::new();
            for arg in args {
                // Each is a var dec.
                let VarDec { name, var_type, .. } = &arg.node else {
                    unreachable!()
                };
                // How to get non-sequential declarations?
                let Some(arg_type) = node_to_type(var_type, current, table) else {
                    return Err(UndefinedType {
                        name: node_type_to_str(var_type),
                    });
                };
                // Add arg to scope.
                table.scopes[idx]
                    .vars
                    .insert(name.clone(), arg_type.clone());
                arg_types.push(arg_type);
            }

            // Add function to parent scope.
            let fn_type = self::Type {
                ty: TypeType::Fn {
                    args: arg_types,
                    ret: match node_to_type(ret_type, idx, table) {
                        Some(ty) => Box::new(ty),
                        None => {
                            return Err(UndefinedType {
                                name: node_type_to_str(ret_type),
                            });
                        }
                    },
                },
                size: 0,
            };
            table.scopes[current]
                .functions
                .insert(name.clone(), fn_type);

            // Check scopes of body.
            scope_expr(table, body, idx)?;
        }

        VarDec {
            name,
            expr,
            var_type,
        } => {
            // Check if var already declared in current scope
            if table.scopes[current].vars.contains_key(name) {
                return Err(AlreadyDeclared { name: name.clone() });
            }

            let ty = node_to_type(var_type, current, table);
            if let Some(expr) = expr {
                scope_expr(table, expr, current)?;
                //inits.insert(name.clone());
            }
            if let Some(ty) = ty {
                table.scopes[current].vars.insert(name.clone(), ty);
            } else {
                return Err(UndefinedType {
                    name: node_type_to_str(var_type),
                });
            }
        }

        StructDec { name, fields } => {
            // Check if struct already declared in current scope
            if table.scopes[current].types.contains_key(name) {
                return Err(AlreadyDeclared { name: name.clone() });
            }

            let mut fields_vec: Vec<(String, self::Type, u32)> = Vec::new();
            let mut offset = 0;
            for field in fields {
                // Each is a var dec.
                let NodeType::VarDec { name, var_type, .. } = &field.node else {
                    unreachable!()
                };

                // Check type.
                let Some(field_type) = node_to_type(var_type, current, table) else {
                    return Err(UndefinedType {
                        name: node_type_to_str(var_type),
                    });
                };

                fields_vec.push((name.clone(), field_type.clone(), offset));
                offset += field_type.size; // Non-C-ABI compat? Change.
            }
            let ty = TypeType::Struct(Struct {
                name: name.clone(),
                fields: fields_vec,
            });
            table.scopes[current]
                .types
                .insert(name.clone(), ty.to_type());
        }

        EnumDec { name, variants } => {
            // Check if enum already declared in current scope
            if table.scopes[current].types.contains_key(name) {
                return Err(AlreadyDeclared { name: name.clone() });
            }

            let mut variants_vec = Vec::new();
            for v in variants {
                // More elegant way to do it?
                let ty = match &v.var_type {
                    Some(ty) => match node_to_type(ty, current, table) {
                        Some(ty) => Some(ty),
                        None => {
                            return Err(UndefinedType {
                                name: node_type_to_str(ty),
                            });
                        }
                    },
                    None => None,
                };
                variants_vec.push((v.name.clone(), ty));
            }
            let ty = TypeType::Enum(Enum {
                name: name.clone(),
                variants: variants_vec,
            });
            table.scopes[current]
                .types
                .insert(name.clone(), ty.to_type());
        }

        _ => {}
    }
    Ok(())
}

// Checks scope of an expression.
fn scope_expr(table: &mut ScopeTable, expr: &Expr, current: usize) -> Result<(), ScopeError> {
    use ExprType::*;
    use ScopeError::*;

    match &expr.expr {
        Var { path } => {
            if table.get_var(path, current).is_none() {
                return Err(UndefinedVar {
                    name: path.base().clone(),
                });
            }
        }

        // TODO:
        // Both match and if have predicates. This split control flow.
        // That conflicts whether or not a variable is initialized.
        // So we need to resolve it between branches.
        // Perhaps done in flow_graph.rs?
        Match { expr, grds } => {
            scope_expr(table, expr, current)?;
            for grd in grds {
                scope_node(table, grd, current)?;
            }
        }
        If {
            pred,
            then,
            else_block,
        } => {
            scope_expr(table, pred, current)?;
            scope_expr(table, then, current)?;
            if let Some(else_block) = else_block {
                scope_expr(table, else_block, current)?;
            }
        }
        Block { lines } => {
            // The only expression that has its own scope!
            // Use it in others if you want...
            let idx = table.new_scope(Some(current), expr.id);
            for node in lines {
                // Declarations.
                register_dec(table, node, idx)?;
            }
            for node in lines {
                // Scopes.
                scope_node(table, node, idx)?;
            }
        }
        FnCall { path, args } => {
            for arg in args {
                scope_expr(table, arg, current)?;
            }
            if table.get_fn(path, current).is_none() {
                return Err(UndefinedFn { name: path.base() });
            }
        }
        Field { base, field } => {
            // TODO: How to scope field???
            // Might pass on as type checker's responsibility.
            scope_expr(table, base, current)?;
        }
        Struct { path, fields } => {
            for field in fields {
                scope_expr(table, field, current)?;
            }
            if table.get_type(path, current).is_none() {
                return Err(UndefinedType { name: path.base() });
            }
        }
        Enum { path, variant, val } => {
            if let Some(val) = val {
                scope_expr(table, val, current)?;
            }

            let Some(Type {
                ty: TypeType::Enum(self::Enum { variants, .. }),
                ..
            }) = table.get_type(path, current)
            else {
                return Err(UndefinedType { name: path.base() });
            };
            // If variant is not found in declaration, return error.
            if !variants.iter().any(|v| v.0 == *variant) {
                return Err(UndefinedEnumVariant {
                    parent: path.base(),
                    name: variant.clone(),
                });
            }
        }

        BinOp { first, second, op } => {
            scope_expr(table, first, current)?;
            scope_expr(table, second, current)?;
            // TODO: If op is assign, add left to initialized hashset.
            // Or is that part of flow_graph.rs?
            // Also a check on whether first is an assignable expr.
        }
        UnOp { expr, .. } => {
            scope_expr(table, expr, current)?;
        }

        Return { val } => {
            if let Some(val) = val {
                scope_expr(table, val, current)?;
            }
        }

        // Don't have values.
        Break => {}
        Continue => {}

        // Always in scope.
        Const { .. } => {}
    }
    Ok(())
}

/*---Tests---*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope() {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::open("./examples/quicksort.zg").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let tokenized = crate::ast::lexer::tokenize_code(&contents);
        let ast = crate::ast::tree::parse_file(tokenized, "test").unwrap();
        println!("Were scopes right?");
        println!("{}", populate_scope(&ast).is_ok());
    }
}
