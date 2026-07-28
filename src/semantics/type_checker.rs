use crate::ast::ast_types::*;
use crate::semantics::types::*;
use std::collections::HashSet;

struct LoopInits {
    breaks: Vec<HashSet<String>>,
    continues: Vec<HashSet<String>>,
    id: Id,
}

struct TypeChecker {
    table: ScopeTable,
    init_set: HashSet<String>,
    loop_inits: Vec<LoopInits>,
}

/*---Helper Functions---*/

fn const_to_type(val: &Constant) -> Type {
    use self::Prim::*;
    use super::types::TypeType::Prim;

    match val {
        Constant::Num(_) => Type {
            ty: Prim(Int64),
            size: 8,
        },
        Constant::Float(_) => Type {
            ty: Prim(Float64),
            size: 8,
        },
        Constant::Bool(_) => Type {
            ty: Prim(Bool),
            size: 1,
        },
        Constant::Char(_) => Type {
            ty: Prim(Char),
            size: 1,
        },
    }
}

/*---Bidirectional Type Checking---*/

// Trying my hand at type inference, and thus elision of type annotations.
// synth/check pair. Assume everything already scoped.

// Infer the type of an expression.
fn synth(expr: &Expr, scope: usize, table: &mut ScopeTable) -> Result<Type, TypeCheckError> {
    use self::Prim::*;
    use ExprKind::*;
    use TypeType::*;

    match &expr.expr {
        Var { path } => {
            let ty = table.get_var(path, scope).unwrap(); // SHOULD be fine if scoping is done before.
            Ok(ty.clone())
        }
        Const { val } => {
            // TODO: Strings. Also other sizes?
            Ok(const_to_type(&val))
        }
        BinOp { first, second, op } => {
            use crate::ast::lexer::Operator::*;

            let first_type = synth(first, scope, table)?;
            let second_type = synth(second, scope, table)?;
            match op {
                Add | Sub | Mul | Div | Exp | Mod => {
                    if first_type.is_float() || second_type.is_float() {
                        Ok(Type {
                            ty: Prim(Float64),
                            size: 8,
                        })
                    } else {
                        Ok(Type {
                            ty: Prim(Int64),
                            size: 8,
                        })
                    }
                }
                LT | GT | ET | LorET | GorET | NotET | Or | And => Ok(Type {
                    ty: Prim(Bool),
                    size: 1,
                }),
                Assign => Ok(Type {
                    ty: second_type.ty,
                    size: second_type.size,
                }),
                _ => unreachable!(),
            }
        }

        UnOp { op, expr } => {
            let expr_type = synth(expr, scope, table)?;
            use crate::ast::lexer::Operator::*;
            match op {
                Inc | Dec => Ok(expr_type),
                Neg => Ok(Type {
                    ty: Prim(Bool),
                    size: 1,
                }),
                Ref => Ok(Type {
                    ty: Prim(self::Prim::Ref(Box::new(expr_type.ty))),
                    size: 8,
                }),
                Deref => {
                    let ty = match expr_type.ty {
                        Prim(self::Prim::Ref(ty)) => *ty,
                        _ => {
                            return Err(TypeCheckError {
                                ty: TypeCheckErrorType::DerefOnNonRef { ty: expr_type },
                                location: expr.span.clone(),
                            });
                        }
                    };
                    let expr_type = Type {
                        ty,
                        size: expr_type.size,
                    };
                    Ok(expr_type)
                }
                _ => unreachable!(),
            }
        }

        FnCall { path, .. } => {
            let fn_type = table.get_fn(path, scope);
            match fn_type {
                Some(Type {
                    ty: TypeType::Fn { ret, .. },
                    ..
                }) => Ok(*ret.clone()),
                _ => unreachable!(),
            }
        }

        Field { base, field } => {
            // This is annoying. If we access the field of an expr evaluating to a struct,
            // we have to check whether it has that field at the type check stage,
            // not scope check stage, because the type of the expression has to be inferred using synth()
            // and using it in scope stage causes breaks with the assurances of scopes
            // we assume we have in synth().
            //
            // TODO: Also, handle modules.

            let ty = synth(base, scope, table)?;
            let Type {
                ty: TypeType::Struct(self::Struct { fields, .. }),
                ..
            } = ty.clone()
            else {
                return Err(TypeCheckError {
                    ty: TypeCheckErrorType::FieldOnNonStruct {
                        ty,
                        field: field.clone(),
                    },
                    location: base.span.clone(),
                });
            };
            match fields.iter().find(|(field_name, _, _)| field_name == field) {
                Some((_, ty, _)) => Ok(ty.clone()),
                None => Err(TypeCheckError {
                    ty: TypeCheckErrorType::FieldNotInStruct {
                        field: field.clone(),
                        ty,
                    },
                    location: base.span.clone(),
                }),
            }
        }

        ExprKind::Struct { path, fields } => {
            // Get the type def, to match fields later.
            let Some(struct_def) = table.get_type(path, scope).cloned() else {
                unreachable!() // Should've checked scope by now. Hopefully!!! Do I call synth in scope check?
            };
            let Type {
                ty:
                    TypeType::Struct(super::types::Struct {
                        fields: struct_fields,
                        ..
                    }),
                ..
            } = struct_def.clone()
            else {
                unreachable!()
            };

            // Check each field to see if types match up.
            for field in fields {
                // Destructure field declaration to get name of field.
                let Expr {
                    expr:
                        ExprKind::BinOp {
                            first: field_var_expr,
                            second: field_expr,
                            ..
                        },
                    ..
                } = field
                else {
                    unreachable!()
                };
                let Expr {
                    expr: Var { path: field_name },
                    ..
                } = &**field_var_expr
                else {
                    unreachable!()
                };

                let ty = synth(&field_expr, scope, table)?;
                let expected = struct_fields
                    .iter()
                    .find(|(name, _, _)| name == &field_name.base())
                    .unwrap() // Scope checker already verified?
                    .1
                    .clone();
                if !check(&expected, &ty) {
                    return Err(TypeCheckError {
                        ty: TypeCheckErrorType::TypeMismatch {
                            expected,
                            actual: ty,
                        },
                        location: field_expr.span.clone(),
                    });
                }
            }

            Ok(struct_def)
        }

        ExprKind::Enum { path, variant, val } => {
            // Return the enum type itself, not any specific variant.
            // But we do checks to make sure the variant is valid.

            // Unwrap because scope check done before in pipeline!
            let ty = table.get_type(path, scope).unwrap().to_owned();
            let Type {
                ty: self::TypeType::Enum(self::Enum { variants, .. }),
                ..
            } = ty.clone()
            else {
                return Err(TypeCheckError {
                    ty: TypeCheckErrorType::VariantOnNonEnum { ty },
                    location: expr.span.clone(),
                });
            };

            // If they're assigning a val, we check to see if it matches.
            if let Some(val) = val {
                let expr_ty = synth(val, scope, table)?;
                let Some((_, Some(variant_ty))) = variants.iter().find(|(n, _)| n == variant)
                else {
                    return Err(TypeCheckError {
                        // We do variant checks in scope pass, so we know variant exists, just blank.
                        ty: TypeCheckErrorType::ValOnBlankVariant {
                            ty,
                            variant: variant.clone(),
                            val: *val.clone(),
                        },
                        location: expr.span.clone(),
                    });
                };
                if !check(&expr_ty, variant_ty) {
                    return Err(TypeCheckError {
                        ty: TypeCheckErrorType::TypeMismatch {
                            expected: variant_ty.clone(),
                            actual: expr_ty,
                        },
                        location: expr.span.clone(),
                    });
                }
            }

            Ok(ty)
        }

        If {
            then, else_block, ..
        } => {
            let then_ty = synth(then, scope, table)?;
            // Do we need this check? Doesn't seem synth's responsibility.
            // What does synth do, compared to check? Infer type. But check
            // specific type doesn't happen elsewhere?
            if let Some(else_block) = else_block {
                let else_ty = synth(else_block, scope, table)?;
                if !check(&then_ty, &else_ty) {
                    return Err(TypeCheckError {
                        ty: TypeCheckErrorType::TypeMismatch {
                            expected: then_ty,
                            actual: else_ty,
                        },
                        location: else_block.span.clone(),
                    });
                }
                if then_ty.ty == TypeType::Prim(self::Prim::Never) {
                    return Ok(else_ty);
                }
            }
            Ok(then_ty)
        }

        // TODO: Refactor this hot mess.
        Match { expr, grds } => {
            if grds.is_empty() {
                return Ok(Type::void());
            }

            // Destruct grds into Vec<(Pattern, Then)>
            let grd_vec: Vec<(Pattern, Expr)> = grds
                .iter()
                .map(|n| {
                    let NodeKind::Guard { patt, expr: then } = &n.node else {
                        unreachable!()
                    };
                    (patt.clone(), then.clone())
                })
                .collect();

            let expr_ty = synth(expr, scope, table)?;

            // If expr is enum, check guards to see if every enum variant is covered.
            // Otherwise, check values to see if they match the type of expression.
            if let TypeType::Enum(self::Enum { variants, .. }) = &expr_ty.ty {
                if grd_vec
                    .iter()
                    .any(|(patt, _)| matches!(patt, Pattern::Val { .. }))
                {
                    return Err(TypeCheckError {
                        ty: TypeCheckErrorType::ValWhenDestructEnum,
                        location: expr.span.clone(),
                    });
                }

                // Destruct patterns to vec of variants.
                let grd_variants: Vec<String> = grd_vec
                    .iter()
                    .map(|(patt, _)| {
                        let Pattern::Variant { name, .. } = patt else {
                            unreachable!()
                        };
                        name.clone()
                    })
                    .collect();
                // Check each variant to see if in type def.
                for v1 in grd_variants.clone() {
                    if !variants.iter().any(|v2| v2.0 == v1) {
                        return Err(TypeCheckError {
                            ty: TypeCheckErrorType::VariantNotInEnum {
                                ty: expr_ty.clone(),
                                variant: v1.clone(),
                            },
                            location: expr.span.clone(),
                        });
                    }
                }
                // Check if all variants are accounted for.
                // First see if there's any catch-all patterns.
                if !grd_vec
                    .iter()
                    .any(|(patt, _)| matches!(patt, Pattern::All | Pattern::Var { .. }))
                {
                    for variant in variants.iter().map(|v| v.0.clone()) {
                        if !grd_variants.iter().any(|v1| v1 == &variant) {
                            return Err(TypeCheckError {
                                ty: TypeCheckErrorType::EnumVariantMissing { expected: variant },
                                location: expr.span.clone(),
                            });
                        }
                    }
                }
            } else {
                // Non-enum, so we check val to see if it matches the type of expression.
                for (patt, _) in grd_vec.clone() {
                    match patt {
                        Pattern::All | Pattern::Var { .. } => break,
                        Pattern::Variant { .. } => {
                            return Err(TypeCheckError {
                                ty: TypeCheckErrorType::UnexpectedVariantPattern {
                                    patt: patt.clone(),
                                },
                                location: expr.span.clone(),
                            });
                        }
                        Pattern::Val { val } => {
                            let placeholder_expr = Expr {
                                expr: ExprKind::Const { val: val.clone() },
                                span: crate::ast::lexer::Span { start: 0, end: 0 },
                                id: Id(0),
                            };
                            let val_ty = synth(&placeholder_expr, scope, table)?;
                            if !check(&expr_ty, &val_ty) {
                                return Err(TypeCheckError {
                                    ty: TypeCheckErrorType::TypeMismatch {
                                        expected: expr_ty,
                                        actual: val_ty,
                                    },
                                    location: expr.span.clone(),
                                });
                            }
                        }
                    }
                }
            }

            // Check all guards to see if types of 'then' blocks are same.
            let NodeKind::Guard {
                expr: first_grd_expr,
                ..
            } = grds[0].node.clone()
            else {
                unreachable!()
            };
            let ret_ty = synth(&first_grd_expr, scope, table)?;
            for (_, then) in grd_vec {
                let then_ty = synth(&then, scope, table)?;
                if !check(&ret_ty, &then_ty) {
                    return Err(TypeCheckError {
                        ty: TypeCheckErrorType::TypeMismatch {
                            expected: ret_ty,
                            actual: then_ty,
                        },
                        location: expr.span.clone(),
                    });
                }
            }

            // Return type of first guard.
            Ok(ret_ty)
        }

        // Difficult one.
        Block { lines } => {
            if lines.is_empty() {
                return Ok(Type::void());
            }

            // If last line of block isn't a statement, return is void.
            let NodeKind::Statement { expr: last_expr } = &lines.last().unwrap().node else {
                return Ok(Type::void());
            };

            // return type of last expression.
            Ok(synth(&last_expr, table.node_scope[&expr.id], table)?)
        }

        // But how to check return type of their value?
        // Like in functions, we check return type via synth on the block, which gets last statement.
        // That's wrong!! Control flow graph needed.
        // Storing both init states and ret types??
        // ERROR:
        Return { .. } | Break { .. } | Continue => Ok(Type::never()),
    }
}

// Check types to see if matching.
fn check(expected: &Type, actual: &Type) -> bool {
    match actual {
        // Never matches anything.
        Type {
            ty: TypeType::Prim(Prim::Never),
            ..
        } => true,

        // Ref types are destructured and checked on their bases.
        Type {
            ty: TypeType::Prim(Prim::Ref(actual_base)),
            ..
        } => {
            let TypeType::Prim(Prim::Ref(expected_base)) = &expected.ty else {
                return false;
            };
            check(
                &expected_base.clone().to_type(),
                &actual_base.clone().to_type(),
            )
        }

        // Match nums/floats separately, check if actual can be upcast.
        actual if actual.is_float() => expected.is_float() && expected.size >= actual.size,
        actual if actual.is_num() => expected.is_num() && expected.size >= actual.size,

        _ => actual == expected,
    }
}
