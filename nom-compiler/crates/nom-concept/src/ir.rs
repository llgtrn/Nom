//! Typed Intermediate Representation — the bridge between parsed AST and LLVM codegen.
//!
//! `IrType` encodes the type system. `IrValue` holds compile-time constants.
//! `IrInstr` is the instruction set for a flat basic-block body. `IrFunction`
//! and `IrModule` are the top-level containers.

// ─── Types ──────────────────────────────────────────────────────────────────

/// The type of a value or parameter at the IR level.
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    /// Zero-size unit type (`()`).
    Unit,
    /// Boolean — 1 bit logical.
    Bool,
    /// Integer of the given bit width (8, 16, 32, 64).
    Int(u32),
    /// IEEE float of the given bit width (32, 64).
    Float(u32),
    /// UTF-8 string slice.
    Str,
    /// Fixed-length array of an element type.
    Array(Box<IrType>, usize),
    /// Shared reference to another type.
    Reference(Box<IrType>),
    /// User-defined type identified by name.
    Named(String),
}

impl IrType {
    /// `true` for `Int` and `Float` variants.
    pub fn is_numeric(&self) -> bool {
        matches!(self, IrType::Int(_) | IrType::Float(_))
    }

    /// `true` for `Unit`, `Bool`, `Int`, `Float`.
    pub fn is_primitive(&self) -> bool {
        matches!(self, IrType::Unit | IrType::Bool | IrType::Int(_) | IrType::Float(_))
    }

    /// Bit width for `Int` and `Float`; `None` for all other variants.
    pub fn size_bits(&self) -> Option<u32> {
        match self {
            IrType::Int(w) | IrType::Float(w) => Some(*w),
            _ => None,
        }
    }
}

// ─── Values ─────────────────────────────────────────────────────────────────

/// A compile-time constant value at the IR level.
#[derive(Debug, Clone)]
pub enum IrValue {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl IrValue {
    /// Returns the `IrType` that corresponds to this value.
    pub fn type_of(&self) -> IrType {
        match self {
            IrValue::Unit => IrType::Unit,
            IrValue::Bool(_) => IrType::Bool,
            IrValue::Int(_) => IrType::Int(64),
            IrValue::Float(_) => IrType::Float(64),
            IrValue::Str(_) => IrType::Str,
        }
    }

    /// `false` only for `Unit` and `Bool(false)`; everything else is truthy.
    ///
    /// This models "has a meaningful value" rather than zero-equality so that
    /// `Int(0)` is considered truthy (the integer zero is still a value).
    pub fn is_truthy(&self) -> bool {
        match self {
            IrValue::Unit => false,
            IrValue::Bool(b) => *b,
            _ => true,
        }
    }
}

// ─── Instructions ────────────────────────────────────────────────────────────

/// A single instruction in a flat IR basic-block body.
#[derive(Debug, Clone)]
pub enum IrInstr {
    /// `dest = value` — assign a constant into a named slot.
    Assign { dest: String, value: IrValue },
    /// Call a named function, optionally capturing the result.
    Call { func: String, args: Vec<IrValue>, dest: Option<String> },
    /// Return a value from the current function.
    Return(IrValue),
    /// Conditional branch: jump to `then_label` if `cond` is truthy, else `else_label`.
    BranchIf { cond: String, then_label: String, else_label: String },
    /// A branch target label.
    Label(String),
    /// No-operation placeholder.
    Nop,
}

// ─── Function ────────────────────────────────────────────────────────────────

/// A named function in the IR with typed parameters and a flat body.
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub body: Vec<IrInstr>,
}

impl IrFunction {
    /// Create an empty function with the given name and return type.
    pub fn new(name: &str, return_type: IrType) -> Self {
        Self {
            name: name.to_owned(),
            params: Vec::new(),
            return_type,
            body: Vec::new(),
        }
    }

    /// Builder-style helper: append a parameter.
    pub fn with_param(mut self, name: &str, ty: IrType) -> Self {
        self.params.push((name.to_owned(), ty));
        self
    }

    /// Builder-style helper: append an instruction to the body.
    pub fn push_instr(mut self, instr: IrInstr) -> Self {
        self.body.push(instr);
        self
    }

    /// Number of instructions in the body.
    pub fn instr_count(&self) -> usize {
        self.body.len()
    }

    /// `true` if the body contains at least one `IrInstr::Return`.
    pub fn has_return(&self) -> bool {
        self.body.iter().any(|i| matches!(i, IrInstr::Return(_)))
    }
}

// ─── Module ──────────────────────────────────────────────────────────────────

/// A named collection of IR functions — the top-level IR container.
#[derive(Debug, Default)]
pub struct IrModule {
    pub name: String,
    pub functions: Vec<IrFunction>,
}

impl IrModule {
    /// Create an empty module with the given name.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_owned(), functions: Vec::new() }
    }

    /// Builder-style helper: append a function.
    pub fn push_function(mut self, func: IrFunction) -> Self {
        self.functions.push(func);
        self
    }

    /// Look up a function by name.
    pub fn get_function(&self, name: &str) -> Option<&IrFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Number of functions in the module.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_type_is_numeric() {
        assert!(IrType::Int(32).is_numeric());
        assert!(IrType::Float(64).is_numeric());
        assert!(!IrType::Bool.is_numeric());
        assert!(!IrType::Str.is_numeric());
        assert!(!IrType::Unit.is_numeric());
    }

    #[test]
    fn ir_type_size_bits() {
        assert_eq!(IrType::Int(32).size_bits(), Some(32));
        assert_eq!(IrType::Float(64).size_bits(), Some(64));
        assert_eq!(IrType::Str.size_bits(), None);
        assert_eq!(IrType::Bool.size_bits(), None);
        assert_eq!(IrType::Unit.size_bits(), None);
    }

    #[test]
    fn ir_value_type_of() {
        assert_eq!(IrValue::Unit.type_of(), IrType::Unit);
        assert_eq!(IrValue::Bool(true).type_of(), IrType::Bool);
        assert_eq!(IrValue::Int(42).type_of(), IrType::Int(64));
        assert_eq!(IrValue::Float(3.14).type_of(), IrType::Float(64));
        assert_eq!(IrValue::Str("hi".into()).type_of(), IrType::Str);
    }

    #[test]
    fn ir_value_is_truthy() {
        assert!(!IrValue::Unit.is_truthy());
        assert!(!IrValue::Bool(false).is_truthy());
        assert!(IrValue::Bool(true).is_truthy());
        // Int(0) is truthy: the zero integer is still a value.
        assert!(IrValue::Int(0).is_truthy());
        assert!(IrValue::Int(42).is_truthy());
        assert!(IrValue::Float(0.0).is_truthy());
        assert!(IrValue::Str(String::new()).is_truthy());
    }

    #[test]
    fn ir_function_new_push_instr_count() {
        let f = IrFunction::new("add", IrType::Int(32))
            .with_param("a", IrType::Int(32))
            .with_param("b", IrType::Int(32))
            .push_instr(IrInstr::Nop)
            .push_instr(IrInstr::Return(IrValue::Int(0)));
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.instr_count(), 2);
    }

    #[test]
    fn ir_function_has_return() {
        let without = IrFunction::new("noop", IrType::Unit)
            .push_instr(IrInstr::Nop);
        assert!(!without.has_return());

        let with_ret = IrFunction::new("answer", IrType::Int(64))
            .push_instr(IrInstr::Return(IrValue::Int(42)));
        assert!(with_ret.has_return());
    }

    #[test]
    fn ir_module_push_function_count() {
        let m = IrModule::new("my_module")
            .push_function(IrFunction::new("foo", IrType::Unit))
            .push_function(IrFunction::new("bar", IrType::Bool));
        assert_eq!(m.function_count(), 2);
        assert_eq!(m.name, "my_module");
    }

    #[test]
    fn ir_module_get_function() {
        let m = IrModule::new("pkg")
            .push_function(IrFunction::new("alpha", IrType::Str))
            .push_function(IrFunction::new("beta", IrType::Int(32)));

        let found = m.get_function("alpha");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "alpha");

        assert!(m.get_function("gamma").is_none());
    }
}
