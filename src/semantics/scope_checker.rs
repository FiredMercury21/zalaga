use crate::ast::ast_types::*;
use crate::semantics::sem_types::*;
use std::collections::{HashMap, HashSet};

// String to primitive type
impl std::str::FromStr for Prim {
    type Err = ();
    fn from_str(ty_str: &str) -> Result<Self, ()> {
        use Prim::*;
        Ok(match ty_str {
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

            _ => return Err(()),
        })
    }
}

// Populates scope table for AST root node, checking
// scope of every node & expression along the way.
pub fn populate_scope(root: &Node) -> Result<ScopeTable, ScopeError> {
    // Init table and scope.
    let mut table = ScopeTable::default();
    let global_id = table.new_scope(None, root.id); // Should be 0.
    let NodeKind::Module { global, .. } = &root.node else {
        unreachable!()
    };

    // Small pass to register types, then vars and functions.
    register_dec(&mut table, global, global_id)?;

    // Check scope of everything.
    for node in global {
        scope_node(&mut table, &node, global_id)?;
    }

    Ok(table)
}

// Converts a NodeType::Type node to a string.
fn node_type_to_str(node: &Node) -> String {
    let NodeKind::Type { name: mut current } = node.node.clone() else {
        unreachable!()
    };
    loop {
        match current {
            TypeNode::Base(path) => {
                return path.base().to_string();
            }
            TypeNode::Ref(inner) => {
                current = *inner;
            }
            TypeNode::Infer => {
                return "infer".to_string();
            }
        }
    }
}

// Traverse type declarations, check no recursive types.
fn check_recursive(lines: &Vec<Node>) -> Result<(), ScopeError> {
    use NodeKind::*;

    // HashMap of Type (String) -> Dependecies in current scope
    let mut type_deps: HashMap<String, (Vec<String>, Span)> = HashMap::new();

    // Types declared in current scope.
    let mut type_strs: HashSet<String> = HashSet::new();
    for line in lines.iter() {
        if let StructDec { name, .. } | EnumDec { name, .. } = &line.node {
            if !type_strs.insert(name.clone()) {
                return Err(ScopeError {
                    kind: ScopeErrorKind::AlreadyDeclared { name: name.clone() },
                    span: line.span(),
                });
            }
        };
    }

    fn is_local_dep(type_strs: &HashSet<String>, ty: &Node) -> Option<String> {
        if let Type {
            name: TypeNode::Base(path),
            ..
        } = &ty.node
        {
            if !path.is_module_path() && type_strs.contains(path.base()) {
                return Some(path.base().to_string());
            }
        }
        None
    }

    for n in lines {
        let (name, (dependencies, span)) = match &n.node {
            StructDec { name, fields } => (
                name.clone(),
                // Find non-reference dependencies in current scope, for recursion check.
                (
                    fields
                        .iter()
                        .filter_map(|field| is_local_dep(&type_strs, &field))
                        .collect::<Vec<String>>(),
                    n.span(),
                ),
            ),
            EnumDec { name, variants } => (
                name.clone(),
                (
                    variants
                        .iter()
                        .filter_map(|variant| variant.var_type.as_deref())
                        .filter_map(|n| is_local_dep(&type_strs, n))
                        .collect::<Vec<String>>(),
                    n.span(),
                ),
            ),
            _ => continue,
        };
        // Insert dependencies to list.
        type_deps.insert(name.clone(), (dependencies, span));
    }

    // 3-Colour graph. Like in rustc?
    // It's called 3-Colour but we only need two; Type not being
    // in the Type->State HashMap functions as a third, 'TBD'.
    enum DepState {
        Doing,
        Done,
    }

    fn recurse_check(
        check: &Vec<String>,
        deps: &HashMap<String, (Vec<String>, Span)>,
        states: &mut HashMap<String, DepState>,
    ) -> Result<(), ScopeError> {
        for dep in check.iter() {
            match states.get(dep) {
                Some(DepState::Doing) => {
                    return Err(ScopeError {
                        kind: ScopeErrorKind::RecursiveType { ty: dep.clone() },
                        span: deps[dep].1.clone(),
                    });
                }
                Some(DepState::Done) => continue,
                _ => {}
            }
            states.insert(dep.clone(), DepState::Doing);
            recurse_check(&deps[dep].0, deps, states)?;
            states.insert(dep.clone(), DepState::Done);
        }
        Ok(())
    }

    let mut states = HashMap::new();
    for ty in &type_strs {
        recurse_check(&type_deps[ty].0, &type_deps, &mut states)?;
    }

    Ok(())
}

fn build_type(
    ty_path: &Path,
    span: Span,
    idx: usize,
    table: &mut ScopeTable,
    decls: &HashMap<String, &Node>,
) -> Result<Type, ScopeError> {
    if ty_path.is_module_path() {
        return table.get_type(ty_path, idx).ok_or(ScopeError {
            kind: ScopeErrorKind::UndefinedType {
                path: ty_path.clone(),
            },
            span,
        });
    }

    fn destruct_structdec(node: &Node) -> (String, Box<Node>) {
        if let NodeKind::VarDec { name, var_type, .. } = &node.node {
            (name.clone(), var_type.clone())
        } else {
            unreachable!()
        }
    }

    fn destruct_enumvar(evar: &EnumVariant) -> (String, Option<Box<Node>>) {
        let EnumVariant { name, var_type } = evar;
        (name.clone(), var_type.clone())
    }

    if let Some(node) = decls.get(&ty_path.base().to_string()) {
        Ok(match &node.node {
            NodeKind::StructDec { name, fields } => TypeKind::Struct(Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|n| destruct_structdec(n))
                    .map(|(name, var_ty)| {
                        node_dec_type(&*var_ty, idx, table, decls).map(|t| (name, t))
                    })
                    .collect::<Result<Vec<(String, Type)>, ScopeError>>()?
                    .into_iter()
                    .scan((String::new(), Type::void(), 0), |(_, _, offset), x| {
                        let ret = (x.0, x.1, *offset);
                        *offset += ret.1.size;
                        Some(ret)
                    })
                    .collect(),
            }),
            NodeKind::EnumDec { name, variants } => TypeKind::Enum(Enum {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|evar| destruct_enumvar(evar))
                    .map(|(name, var_type)| {
                        var_type
                            .clone()
                            .map(|t| node_dec_type(&*t, idx, table, decls))
                            .transpose()
                            .map(|t| (name.clone(), t))
                    })
                    .collect::<Result<Vec<(String, Option<Type>)>, ScopeError>>()?,
            }),
            _ => unreachable!(),
        }
        .to_type())
    } else {
        return Err(ScopeError {
            kind: ScopeErrorKind::UndefinedType {
                path: ty_path.clone(),
            },
            span,
        });
    }
}

pub fn node_to_type(node: &Node, idx: usize, table: &mut ScopeTable) -> Result<Type, ScopeError> {
    node_dec_type(node, idx, table, &HashMap::new())
}

fn node_dec_type(
    node: &Node,
    idx: usize,
    table: &mut ScopeTable,
    decls: &HashMap<String, &Node>,
) -> Result<Type, ScopeError> {
    let NodeKind::Type { name } = &node.node else {
        unreachable!()
    };
    let mut current = name;
    let mut base;
    let mut ref_n = 0;

    loop {
        match current {
            TypeNode::Base(path) => {
                if let Ok(ty) = path.base().parse::<Prim>() {
                    base = ty.into();
                } else if decls.get(&node_type_to_str(node)).is_some() {
                    return build_type(path, node.span(), idx, table, decls);
                } else {
                    base = table.get_type(path, idx).ok_or(ScopeError {
                        kind: ScopeErrorKind::UndefinedType { path: path.clone() },
                        span: node.span(),
                    })?;
                }
                break;
            }
            TypeNode::Ref(inner) => {
                ref_n += 1;
                current = &**inner;
            }
            TypeNode::Infer => {
                base = self::Type::infer();
                break;
            }
        }
    }
    let mut output = base;
    for _ in 0..ref_n {
        output = Prim::Ref(Box::new(output.ty)).into()
    }
    Ok(output)
}

// Checks scope of a node!
fn scope_node(table: &mut ScopeTable, node: &Node, current: usize) -> Result<(), ScopeError> {
    use NodeKind::*;

    match &node.node {
        // Handled by other guards using register_dec() on blocks.
        StructDec { .. } | EnumDec { .. } | Use { .. } => {}
        VarDec { expr, .. } => {
            if let Some(expr) = expr {
                scope_expr(table, &expr, current)?;
            }
        }

        FnDec { body, args, .. } => {
            let idx = table.new_scope(Some(current), node.id);
            for arg in args {
                let VarDec { name, var_type, .. } = &arg.node else {
                    unreachable!()
                };
                let arg_ty = node_to_type(&*var_type, idx, table)?;
                table.scopes[idx].vars.insert(name.clone(), arg_ty);
            }
            scope_expr(table, &body, idx)?;
        }

        Statement { expr } => {
            scope_expr(table, &expr, current)?;
        }

        Guard { patt, expr } => {
            use Pattern::*;

            // Guard has to have its own scope, because then the expressions
            // underneath will have access to bound variables.
            let idx = table.new_scope(Some(current), node.id);

            match patt {
                All | Val { .. } => scope_expr(table, &expr, idx)?,
                Variant { payload: var, .. } | Var { name: var } => {
                    // For now, put the placeholder type of Never on the var.
                    // It shouldn't cause any issues? We check types later.
                    table.scopes[idx]
                        .vars
                        .insert(var.clone(), self::Type::infer());
                    scope_expr(table, &expr, idx)?;
                }
            }
        }

        For {
            init,
            pred,
            then,
            block,
        } => {
            let idx = table.new_scope(Some(current), node.id);

            // Add var to for block scope.
            if let NodeKind::VarDec { name, var_type, .. } = &init.node {
                let ty = node_to_type(&**var_type, idx, table)?;
                table.scopes[idx].vars.insert(name.clone(), ty);
            } else {
                unreachable!()
            }

            // Check scopes underneath.
            scope_expr(table, &pred, idx)?;
            scope_expr(table, &then, idx)?;
            scope_expr(table, &block, idx)?;
        }

        While { pred, block } => {
            scope_expr(table, &pred, current)?;
            scope_expr(table, &block, current)?;
        }

        Type { .. } => unreachable!(),
        Module { .. } => unreachable!(),
    }
    Ok(())
}

fn register_modules(
    table: &mut ScopeTable,
    lines: &Vec<Node>,
    current: usize,
) -> Result<(), ScopeError> {
    use NodeKind::*;
    use ScopeErrorKind::*;

    for line in lines {
        match &line.node {
            Use { name, root } => {
                if table.scopes[current].modules.contains_key(name) {
                    return Err(ScopeError {
                        kind: AlreadyDeclared { name: name.clone() },
                        span: line.span(),
                    });
                }
                let module_scope = match populate_scope(root) {
                    Ok(module_scope) => module_scope,
                    Err(e) => {
                        return Err(ScopeError {
                            kind: ErrInModule {
                                err: Box::new(e),
                                mod_name: name.clone(),
                            },
                            span: line.span(),
                        });
                    }
                };
                table.scopes[current]
                    .modules
                    .insert(name.clone(), module_scope);
            }
            _ => {}
        }
    }
    Ok(())
}

// If given node is declaration, add it to scope. Else, ignore.
fn register_type_dec(
    table: &mut ScopeTable,
    lines: &Vec<Node>,
    current: usize,
) -> Result<(), ScopeError> {
    use NodeKind::*;
    use ScopeErrorKind::*;
    let mut types = HashMap::new();

    for line in lines {
        match &line.node {
            StructDec { name, .. } | EnumDec { name, .. } => {
                if types.insert(name.clone(), line).is_some() {
                    return Err(ScopeError {
                        kind: AlreadyDeclared { name: name.clone() },
                        span: line.span(),
                    });
                }
            }

            _ => {}
        }
    }
    // Build types and add to scope.
    for (ty_name, ty) in &types {
        let t = build_type(&ty_name.as_str().into(), ty.span(), current, table, &types)?;
        table.scopes[current].types.insert(ty_name.clone(), t);
    }
    Ok(())
}

fn find_type_decs(lines: &Vec<Node>) -> HashMap<String, &Node> {
    let mut ty_decls = HashMap::new();
    for line in lines {
        match &line.node {
            NodeKind::StructDec { name, .. } | NodeKind::EnumDec { name, .. } => {
                ty_decls.insert(name.clone(), line);
            }
            _ => {}
        }
    }
    ty_decls
}

fn register_var_fn_dec(
    table: &mut ScopeTable,
    lines: &Vec<Node>,
    current: usize,
    ty_decls: &HashMap<String, &Node>,
) -> Result<(), ScopeError> {
    use NodeKind::*;
    use ScopeErrorKind::*;
    for node in lines {
        match &node.node {
            FnDec {
                name,
                args,
                ret_type,
                ..
            } => {
                // Check if function already declared in current scope
                if table.scopes[current].functions.contains_key(name) {
                    return Err(ScopeError {
                        kind: AlreadyDeclared { name: name.clone() },
                        span: node.span(),
                    });
                }

                // Add each arg to FnType.
                let arg_types = args
                    .iter()
                    .map(|arg| {
                        let VarDec { var_type, .. } = &arg.node else {
                            unreachable!()
                        };
                        node_dec_type(&var_type, current, table, ty_decls)
                    })
                    .collect::<Result<Vec<self::Type>, ScopeError>>()?;

                // Add function to parent scope.
                let fn_type = TypeKind::Fn {
                    args: arg_types,
                    ret: Box::new(node_dec_type(&ret_type, current, table, ty_decls)?),
                }
                .to_type();
                table.scopes[current]
                    .functions
                    .insert(name.clone(), fn_type);
            }

            VarDec { name, var_type, .. } => {
                // Check if var already declared in current scope
                if table.scopes[current].vars.contains_key(name) {
                    return Err(ScopeError {
                        kind: AlreadyDeclared { name: name.clone() },
                        span: node.span(),
                    });
                }

                let ty = node_dec_type(&var_type, current, table, ty_decls)?;
                table.scopes[current].vars.insert(name.clone(), ty);
            }

            _ => {}
        }
    }
    Ok(())
}

fn register_dec(
    table: &mut ScopeTable,
    lines: &Vec<Node>,
    current: usize,
) -> Result<(), ScopeError> {
    register_modules(table, lines, current)?;
    check_recursive(lines)?;
    register_type_dec(table, lines, current)?;
    let type_decls = find_type_decs(lines);
    register_var_fn_dec(table, lines, current, &type_decls)?;
    Ok(())
}

// Checks scope of an expression.
fn scope_expr(table: &mut ScopeTable, expr: &Expr, current: usize) -> Result<(), ScopeError> {
    use ExprKind::*;
    use ScopeErrorKind::*;

    match &expr.expr {
        Var { path } => {
            if table.get_var(path, current).is_none() {
                return Err(ScopeError {
                    kind: UndefinedVar { path: path.clone() },
                    span: expr.span(),
                });
            }
        }
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
            register_dec(table, lines, idx)?;
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
                return Err(ScopeError {
                    kind: UndefinedFn { path: path.clone() },
                    span: expr.span(),
                });
            }
        }
        Field { base, field } => {
            // TODO: How to scope field???
            // Might pass on as type checker's responsibility.
            scope_expr(table, base, current)?;
        }
        Struct { path, fields } => {
            for (_, field_expr) in fields {
                scope_expr(table, field_expr, current)?;
            }
            // TODO: Easier, check structdec and see if all fields exist.
            // Or is that init checker responsibility?
            if table.get_type(path, current).is_none() {
                return Err(ScopeError {
                    kind: UndefinedType { path: path.clone() },
                    span: expr.span(),
                });
            }
        }
        Enum { path, variant, val } => {
            if let Some(val) = val {
                scope_expr(table, val, current)?;
            }

            let Some(Type {
                ty: TypeKind::Enum(self::Enum { variants, .. }),
                ..
            }) = table.get_type(path, current)
            else {
                return Err(ScopeError {
                    kind: UndefinedType { path: path.clone() },
                    span: expr.span(),
                });
            };
            // If variant is not found in declaration, return error.
            if !variants.iter().any(|v| v.0 == *variant) {
                return Err(ScopeError {
                    kind: UndefinedEnumVariant {
                        parent: path.base().to_string(),
                        name: variant.clone(),
                    },
                    span: expr.span(),
                });
            }
        }

        BinOp { first, second, op } => {
            scope_expr(table, first, current)?;
            scope_expr(table, second, current)?;
            // TODO: If op is assign, add left to initialized hashset.
            // Or is that part of flow_graph.rs?
            // Also a check on whether first is an lvalue.
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
    use crate::diagnostics::PathCache;

    #[test]
    fn test_scope() {
        use crate::cli::utils::*;

        let file_str = load_file(&"examples/quicksort.zg".parse().expect("PathBuf"))
            .expect("Unwrap file failed.");
        let mut cache = PathCache::new();
        let ast = verifiers::scope_check(&file_str, "quicksort", &mut cache);
        println!("{:#?}", ast);
    }
}
