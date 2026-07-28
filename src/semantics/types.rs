use crate::ast::ast_types::*;
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
    //String, // TODO:
    Void,
    Never,
    Ref(Box<TypeType>),
}

impl Prim {
    // Size of primitives in bytes.
    fn size_prim(&self) -> u32 {
        use Prim::*;
        match self {
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeType {
    Prim(Prim),
    //Array(Box<Type>), // Pointers bby!
    Struct(Struct),
    Enum(Enum),
    Module(Module),
    Fn { args: Vec<Type>, ret: Box<Type> },
}

// Sometimes clashes with ast_types::NodeType::Type
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub ty: TypeType,
    pub size: u32, // Bytes. Problems if more than a couple GB?
}

// self::Module clashes with ast_types::NodeType::Module.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub table: ScopeTable,
}

// self::{Struct, Enum} have clashes with ast_types::ExprType::{Struct, Enum}.
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<(String, Type, u32)>, // (name, type, byte offset)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<(String, Option<Type>)>,
}

impl Type {
    pub fn void() -> Self {
        Self {
            ty: TypeType::Prim(Prim::Void),
            size: 0,
        }
    }

    pub fn never() -> Self {
        Self {
            ty: TypeType::Prim(Prim::Never),
            size: 0,
        }
    }

    pub fn is_void(&self) -> bool {
        matches!(self.ty, TypeType::Prim(Prim::Void))
    }

    pub fn is_never(&self) -> bool {
        matches!(self.ty, TypeType::Prim(Prim::Never))
    }

    pub fn is_ref(&self) -> bool {
        matches!(self.ty, TypeType::Prim(Prim::Ref(_)))
    }

    pub fn is_num(&self) -> bool {
        use self::Prim::*;
        use TypeType::*;
        matches!(
            self.ty,
            Prim(Char | Bool | Int16 | Int32 | Int64 | Float16 | Float32 | Float64)
        )
    }

    pub fn is_float(&self) -> bool {
        use self::Prim::*;
        use TypeType::*;
        matches!(self.ty, Prim(Float16 | Float32 | Float64))
    }
}

impl TypeType {
    pub fn to_type(self) -> Type {
        let size = self.size_type();
        Type { ty: self, size }
    }

    // Size of a type in bytes.
    fn size_type(&self) -> u32 {
        use TypeType::*;

        match self {
            Prim(prim) => prim.size_prim(),
            Struct(self::Struct { fields, .. }) => fields
                .iter()
                .map(|(_, Type { size, .. }, _)| size) // Should we recurse or trust upstream?
                .sum(),
            Enum(self::Enum { variants, .. }) => {
                variants
                    .iter()
                    .map(|(_, var_ty)| {
                        if let Some(Type { size, .. }) = var_ty {
                            *size
                        } else {
                            0
                        }
                    })
                    .max()
                    .unwrap_or(0)
                    + 1 // One byte for the variant index. Needed?
            }
            Fn { .. } => 8, // Function pointers???
            Module(_) => 0, // Not sized.
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckError {
    pub ty: TypeCheckErrorKind,
    pub location: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeCheckErrorKind {
    ExpectedType {
        expected: Type,
        actual: Type,
    },
    FnNotFound {
        name: String,
    },
    FieldNotFound {
        name: String,
    },
    TypeMismatch {
        expected: Type,
        actual: Type,
    },
    VariantOnNonEnum {
        ty: Type,
    },
    VariantNotInEnum {
        ty: Type,
        variant: String,
    },
    // Want this to contain variants not found.
    EnumVariantMissing {
        expected: String,
    },
    UnexpectedVariantPattern {
        patt: Pattern,
    },
    ValOnBlankVariant {
        ty: Type,
        variant: String,
        val: Expr,
    },
    ValWhenDestructEnum,
    FieldOnNonStruct {
        ty: Type,
        field: String,
    },
    FieldNotInStruct {
        ty: Type,
        field: String,
    },
    DerefOnNonRef {
        ty: Type,
    },
    ErrorInModule {
        err: Box<TypeCheckError>,
        mod_name: String,
    },
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct ScopeError {
//     pub kind: ScopeErrorKind,
//     pub location: Span,
// }

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeError {
    UndefinedType {
        name: String,
    },
    UndefinedVar {
        name: String,
    },
    UndefinedFn {
        name: String,
    },
    UndefinedField {
        field: String,
    },
    UndefinedEnumVariant {
        parent: String,
        name: String,
    },
    AlreadyDeclared {
        name: String,
    },
    ErrInModule {
        err: Box<ScopeError>,
        mod_name: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub parent: Option<usize>, // Vector-tree index.
    pub vars: HashMap<String, Type>,
    pub types: HashMap<String, Type>,
    pub functions: HashMap<String, Type>,
    pub modules: HashMap<String, ScopeTable>,
    pub node: Id,
}

pub struct InitializedVars(pub std::collections::HashSet<String>);

// Flat vector-tree. Why this now, instead of the box approach earlier?
// Because I was dumb earlier, fuck you.
// Man, would be easier if I could just access nodes as usize.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeTable {
    pub scopes: Vec<Scope>,             // indexed by scope id
    pub node_scope: HashMap<Id, usize>, // node id -> scope id
}

enum ScopeType {
    Vars,
    Types,
    Functions,
}

impl Scope {
    pub fn new(parent: Option<usize>, id: Id) -> Self {
        Self {
            parent,
            vars: HashMap::<String, self::Type>::new(),
            types: HashMap::<String, self::Type>::new(),
            functions: HashMap::<String, self::Type>::new(),
            modules: HashMap::<String, ScopeTable>::new(),
            node: id,
        }
    }
}

impl ScopeTable {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            node_scope: HashMap::new(),
        }
    }

    // Adds a new scope to the table and associates it with given node ID.
    pub fn new_scope(&mut self, parent: Option<usize>, id: Id) -> usize {
        self.scopes.push(Scope::new(parent, id));
        let idx = self.scopes.len() - 1;
        self.node_scope.insert(id, idx);
        idx
    }

    fn find_in_scope(&self, path: &Path, current: usize, ty: ScopeType) -> Option<&Type> {
        let mut current_id = current;
        let mut current_scope = &self.scopes[current_id];
        if path.is_module_path() {
            for segment in &path.0 {
                if let Some(module_scope) = self.get_module(&segment, current_id) {
                    current_scope = &module_scope.scopes[0];
                } else {
                    return None;
                }
            }
        }
        loop {
            let name = &path.base();
            match ty {
                ScopeType::Vars => {
                    if current_scope.vars.contains_key(name) {
                        return Some(&current_scope.vars[name]);
                    }
                }
                ScopeType::Types => {
                    if current_scope.types.contains_key(name) {
                        return Some(&current_scope.types[name]);
                    }
                }
                ScopeType::Functions => {
                    if current_scope.functions.contains_key(name) {
                        return Some(&current_scope.functions[name]);
                    }
                }
            }
            // Go to parent scope.
            current_id = current_scope.parent?;
            current_scope = &self.scopes[current_id];
        }
    }

    // Checks scope and all parent scopes to see if the fn/var/type is defined.
    pub fn get_var(&self, path: &Path, current: usize) -> Option<&Type> {
        self.find_in_scope(path, current, ScopeType::Vars)
    }
    pub fn get_type(&self, path: &Path, current: usize) -> Option<&Type> {
        self.find_in_scope(path, current, ScopeType::Types)
    }
    pub fn get_fn(&self, path: &Path, current: usize) -> Option<&Type> {
        self.find_in_scope(path, current, ScopeType::Functions)
    }

    pub fn get_module(&self, name: &str, current: usize) -> Option<&ScopeTable> {
        let mut current_scope = &self.scopes[current];
        loop {
            if current_scope.modules.contains_key(name) {
                return Some(&current_scope.modules[name]);
            }
            // Go to parent scope.
            match current_scope.parent {
                Some(parent) => current_scope = &self.scopes[parent],
                None => return None,
            }
        }
    }
}
