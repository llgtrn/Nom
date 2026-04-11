# nom-llvm Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `nom-llvm` crate that compiles Nom's imperative core (fn, let, if/else, for, while, match, struct, enum, expressions) directly to LLVM IR bitcode (.bc), bypassing Rust codegen entirely.

**Architecture:** The `nom-llvm` crate takes a `CompositionPlan` (same input as `nom-codegen`) and walks the `imperative_stmts` in each `FlowPlan`. It uses `inkwell` (safe LLVM C API bindings) to emit LLVM IR. A minimal runtime library (`nom_runtime`) provides string/print/alloc stubs. The CLI gains `--target llvm` and `--target native` flags.

**Tech Stack:** Rust, inkwell 0.5 (LLVM 18), nom-ast, nom-planner

---

## File Structure

```
nom-compiler/crates/nom-llvm/
├── Cargo.toml              # inkwell dep, nom-ast dep, nom-planner dep
├── src/
│   ├── lib.rs              # Public API: compile(plan) → Result<LlvmOutput>
│   ├── context.rs          # NomCompiler struct wrapping inkwell Context/Module/Builder
│   ├── types.rs            # TypeExpr → inkwell BasicTypeEnum mapping
│   ├── functions.rs        # FnDef → LLVM function compilation
│   ├── statements.rs       # LetStmt, IfExpr, ForStmt, WhileStmt, MatchExpr → IR
│   ├── expressions.rs      # Expr → inkwell BasicValueEnum
│   ├── structs.rs          # StructDef → named LLVM struct types
│   ├── enums.rs            # EnumDef → tagged union (i8 tag + payload)
│   └── runtime.rs          # Declare external runtime stubs (nom_print, etc.)
```

Existing files modified:
- `nom-compiler/Cargo.toml` — add `nom-llvm` to workspace members
- `nom-compiler/crates/nom-cli/Cargo.toml` — add `nom-llvm` dependency
- `nom-compiler/crates/nom-cli/src/main.rs` — add `--target llvm/native` flag to `build` command

---

### Task 1: Create nom-llvm crate scaffold + inkwell dependency

**Files:**
- Create: `nom-compiler/crates/nom-llvm/Cargo.toml`
- Create: `nom-compiler/crates/nom-llvm/src/lib.rs`
- Modify: `nom-compiler/Cargo.toml` (workspace members list)

- [ ] **Step 1: Create Cargo.toml for nom-llvm**

```toml
[package]
name = "nom-llvm"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
nom-ast = { path = "../nom-ast" }
nom-planner = { path = "../nom-planner" }
inkwell = { version = "0.5", features = ["llvm18-0"] }
thiserror.workspace = true
```

- [ ] **Step 2: Create minimal lib.rs with compile entry point**

```rust
//! nom-llvm: LLVM IR backend for the Nom compiler.
//!
//! Compiles Nom's imperative core (fn, struct, enum, control flow)
//! directly to LLVM IR bitcode (.bc). No Rust middle layer.

mod context;
mod expressions;
mod functions;
mod runtime;
mod statements;
mod structs;
mod enums;
mod types;

use nom_planner::CompositionPlan;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlvmError {
    #[error("LLVM compilation error: {0}")]
    Compilation(String),
    #[error("unsupported AST node for LLVM backend: {0}")]
    Unsupported(String),
    #[error("type error: {0}")]
    Type(String),
    #[error("LLVM verification failed: {0}")]
    Verification(String),
}

/// Output from LLVM compilation.
pub struct LlvmOutput {
    /// LLVM IR as human-readable text (.ll format)
    pub ir_text: String,
    /// LLVM bitcode bytes (.bc format)
    pub bitcode: Vec<u8>,
}

/// Compile a CompositionPlan to LLVM IR.
pub fn compile(plan: &CompositionPlan) -> Result<LlvmOutput, LlvmError> {
    let compiler = context::NomCompiler::new();
    compiler.compile_plan(plan)
}
```

- [ ] **Step 3: Create stub files for each module**

Create empty stub files so the crate compiles:

`context.rs`:
```rust
use crate::{LlvmError, LlvmOutput};
use nom_planner::CompositionPlan;

pub struct NomCompiler {
    // Will hold inkwell Context, Module, Builder
}

impl NomCompiler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn compile_plan(&self, _plan: &CompositionPlan) -> Result<LlvmOutput, LlvmError> {
        Ok(LlvmOutput {
            ir_text: String::new(),
            bitcode: Vec::new(),
        })
    }
}
```

`types.rs`:
```rust
// Nom TypeExpr → LLVM type mapping
```

`functions.rs`:
```rust
// FnDef → LLVM function compilation
```

`statements.rs`:
```rust
// Statement → LLVM IR instruction sequences
```

`expressions.rs`:
```rust
// Expr → LLVM IR values
```

`structs.rs`:
```rust
// StructDef → LLVM named struct types
```

`enums.rs`:
```rust
// EnumDef → LLVM tagged unions
```

`runtime.rs`:
```rust
// External runtime function declarations
```

- [ ] **Step 4: Add nom-llvm to workspace members**

In `nom-compiler/Cargo.toml`, add `"crates/nom-llvm"` to the `members` list.

- [ ] **Step 5: Verify the crate compiles**

Run: `cd nom-compiler && cargo check -p nom-llvm`
Expected: Compiles successfully with no errors (warnings OK).

- [ ] **Step 6: Commit**

```bash
git add nom-compiler/crates/nom-llvm/ nom-compiler/Cargo.toml
git commit -m "feat: scaffold nom-llvm crate with inkwell dependency"
```

---

### Task 2: Implement NomCompiler context with inkwell

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/context.rs`
- Modify: `nom-compiler/crates/nom-llvm/src/runtime.rs`
- Test: `nom-compiler/crates/nom-llvm/src/context.rs` (inline test)

- [ ] **Step 1: Write failing test for empty module compilation**

Add to bottom of `context.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nom_planner::CompositionPlan;

    #[test]
    fn empty_plan_produces_valid_ir() {
        let plan = CompositionPlan {
            source_path: Some("test.nom".into()),
            flows: vec![],
            nomiz: "{}".into(),
        };
        let compiler = NomCompiler::new();
        let output = compiler.compile_plan(&plan).unwrap();
        assert!(output.ir_text.contains("source_filename"));
        assert!(!output.bitcode.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd nom-compiler && cargo test -p nom-llvm -- empty_plan_produces_valid_ir`
Expected: FAIL — ir_text is empty, bitcode is empty.

- [ ] **Step 3: Implement NomCompiler with inkwell Context/Module/Builder**

Replace `context.rs` with:
```rust
use crate::runtime::declare_runtime_functions;
use crate::{LlvmError, LlvmOutput};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::FunctionValue;
use inkwell::types::BasicTypeEnum;
use nom_ast::Statement;
use nom_planner::{CompositionPlan, FlowPlan};
use std::collections::HashMap;

/// Holds all LLVM state for a single compilation unit.
pub struct NomCompiler {
    context: Context,
}

/// Per-module compilation state (borrows from NomCompiler's context).
pub struct ModuleCompiler<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    /// Named values in the current scope (variable name → alloca pointer).
    pub named_values: HashMap<String, inkwell::values::PointerValue<'ctx>>,
    /// Named struct types.
    pub struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    /// Function declarations.
    pub functions: HashMap<String, FunctionValue<'ctx>>,
}

impl NomCompiler {
    pub fn new() -> Self {
        Self {
            context: Context::create(),
        }
    }

    pub fn compile_plan(&self, plan: &CompositionPlan) -> Result<LlvmOutput, LlvmError> {
        let module_name = plan
            .source_path
            .as_deref()
            .unwrap_or("nom_module");
        let module = self.context.create_module(module_name);
        let builder = self.context.create_builder();

        // Set source filename for debugging
        module.set_source_file_name(module_name);

        let mut mc = ModuleCompiler {
            context: &self.context,
            module,
            builder,
            named_values: HashMap::new(),
            struct_types: HashMap::new(),
            functions: HashMap::new(),
        };

        // Declare runtime functions (nom_print, etc.)
        declare_runtime_functions(&mut mc);

        // Compile each flow's imperative statements
        for flow in &plan.flows {
            mc.compile_flow(flow)?;
        }

        // Verify the module
        mc.module
            .verify()
            .map_err(|e| LlvmError::Verification(e.to_string()))?;

        // Extract IR text and bitcode
        let ir_text = mc.module.print_to_string().to_string();
        let bitcode = mc.module.write_bitcode_to_memory().as_slice().to_vec();

        Ok(LlvmOutput { ir_text, bitcode })
    }
}

impl<'ctx> ModuleCompiler<'ctx> {
    pub fn compile_flow(&mut self, flow: &FlowPlan) -> Result<(), LlvmError> {
        for stmt in &flow.imperative_stmts {
            self.compile_top_level_statement(stmt)?;
        }
        Ok(())
    }

    pub fn compile_top_level_statement(&mut self, stmt: &Statement) -> Result<(), LlvmError> {
        match stmt {
            Statement::FnDef(fn_def) => {
                crate::functions::compile_fn(self, fn_def)?;
            }
            Statement::StructDef(struct_def) => {
                crate::structs::compile_struct(self, struct_def)?;
            }
            Statement::EnumDef(enum_def) => {
                crate::enums::compile_enum(self, enum_def)?;
            }
            other => {
                return Err(LlvmError::Unsupported(format!(
                    "top-level statement: {:?}",
                    std::mem::discriminant(other)
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_planner::CompositionPlan;

    #[test]
    fn empty_plan_produces_valid_ir() {
        let plan = CompositionPlan {
            source_path: Some("test.nom".into()),
            flows: vec![],
            nomiz: "{}".into(),
        };
        let compiler = NomCompiler::new();
        let output = compiler.compile_plan(&plan).unwrap();
        assert!(output.ir_text.contains("source_filename"));
        assert!(!output.bitcode.is_empty());
    }
}
```

- [ ] **Step 4: Implement runtime function declarations**

Replace `runtime.rs` with:
```rust
use crate::context::ModuleCompiler;
use inkwell::AddressSpace;

/// Declare external runtime functions that Nom programs can call.
/// These are implemented in a separate runtime library linked at build time.
pub fn declare_runtime_functions(mc: &mut ModuleCompiler) {
    let i64_type = mc.context.i64_type();
    let i8_ptr_type = mc.context.ptr_type(AddressSpace::default());
    let void_type = mc.context.void_type();

    // nom_print(text_ptr: *const i8, text_len: i64) -> void
    let print_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let print_fn = mc.module.add_function("nom_print", print_type, None);
    mc.functions.insert("nom_print".into(), print_fn);

    // nom_alloc(size: i64) -> *mut i8
    let alloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    let alloc_fn = mc.module.add_function("nom_alloc", alloc_type, None);
    mc.functions.insert("nom_alloc".into(), alloc_fn);
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd nom-compiler && cargo test -p nom-llvm -- empty_plan_produces_valid_ir`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/
git commit -m "feat(nom-llvm): inkwell context, module compiler, runtime stubs"
```

---

### Task 3: Implement type mapping (Nom TypeExpr → LLVM types)

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/types.rs`
- Test: inline in `types.rs`

- [ ] **Step 1: Write failing test for type resolution**

Add to `types.rs`:
```rust
use crate::context::ModuleCompiler;
use inkwell::types::BasicTypeEnum;
use nom_ast::{Identifier, Span, TypeExpr};

/// Convert a Nom TypeExpr to an LLVM BasicTypeEnum.
pub fn resolve_type<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    type_expr: &TypeExpr,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    todo!()
}

/// Resolve a type name string (used for untyped params that default).
pub fn resolve_type_name<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    name: &str,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    #[test]
    fn number_maps_to_f64() {
        let ctx = Context::create();
        let module = ctx.create_module("test");
        let builder = ctx.create_builder();
        let mc = ModuleCompiler {
            context: &ctx,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        };
        let ty = TypeExpr::Named(Identifier::new("number", Span::default()));
        let llvm_ty = resolve_type(&mc, &ty).unwrap();
        assert!(llvm_ty.is_float_type());
    }

    #[test]
    fn bool_maps_to_i1() {
        let ctx = Context::create();
        let module = ctx.create_module("test");
        let builder = ctx.create_builder();
        let mc = ModuleCompiler {
            context: &ctx,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        };
        let ty = TypeExpr::Named(Identifier::new("bool", Span::default()));
        let llvm_ty = resolve_type(&mc, &ty).unwrap();
        assert!(llvm_ty.is_int_type());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd nom-compiler && cargo test -p nom-llvm -- maps_to`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement type resolution**

Replace the `todo!()` bodies:
```rust
pub fn resolve_type<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    type_expr: &TypeExpr,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    match type_expr {
        TypeExpr::Named(ident) => resolve_type_name(mc, &ident.name),
        TypeExpr::Generic(ident, _args) => {
            // For now, generic types like list[T] are treated as opaque pointers
            match ident.name.as_str() {
                "list" | "vec" => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
                "option" => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
                "map" => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
                _ => Err(crate::LlvmError::Type(format!(
                    "unknown generic type: {}",
                    ident.name
                ))),
            }
        }
        TypeExpr::Unit => {
            // Unit maps to i8 (LLVM void can't be used as a value type)
            Ok(mc.context.i8_type().into())
        }
        TypeExpr::Tuple(_) => {
            // Tuples map to opaque pointers for now
            Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into())
        }
        TypeExpr::Ref { inner, .. } => {
            // References map to pointers
            Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into())
        }
        TypeExpr::Function { .. } => {
            // Function types map to function pointers
            Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into())
        }
    }
}

pub fn resolve_type_name<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    name: &str,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    match name {
        // number / f64 → double
        "number" | "f64" | "float" | "real" => Ok(mc.context.f64_type().into()),
        // integer / i64 → i64
        "integer" | "i64" | "int" => Ok(mc.context.i64_type().into()),
        // i32
        "i32" => Ok(mc.context.i32_type().into()),
        // bool → i1
        "bool" | "yes" | "no" => Ok(mc.context.bool_type().into()),
        // text / String → pointer (fat pointer handled at usage site)
        "text" | "string" | "String" => {
            Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into())
        }
        // bytes → pointer
        "bytes" => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
        // Check if it's a known struct type
        _ => {
            if let Some(struct_ty) = mc.struct_types.get(name) {
                Ok((*struct_ty).into())
            } else {
                Err(crate::LlvmError::Type(format!("unknown type: {}", name)))
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd nom-compiler && cargo test -p nom-llvm -- maps_to`
Expected: PASS (both `number_maps_to_f64` and `bool_maps_to_i1`)

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/types.rs
git commit -m "feat(nom-llvm): type mapping — Nom TypeExpr to LLVM types"
```

---

### Task 4: Implement struct compilation

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/structs.rs`
- Test: inline in `structs.rs`

- [ ] **Step 1: Write failing test**

```rust
use crate::context::ModuleCompiler;
use nom_ast::{Identifier, Span, StructDef, StructField, TypeExpr};

pub fn compile_struct(
    mc: &mut ModuleCompiler,
    struct_def: &StructDef,
) -> Result<(), crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    fn make_mc(ctx: &Context) -> ModuleCompiler {
        ModuleCompiler {
            context: ctx,
            module: ctx.create_module("test"),
            builder: ctx.create_builder(),
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn compiles_point_struct() {
        let ctx = Context::create();
        let mut mc = make_mc(&ctx);
        let point = StructDef {
            name: Identifier::new("Point", Span::default()),
            fields: vec![
                StructField {
                    name: Identifier::new("x", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                    is_pub: false,
                },
                StructField {
                    name: Identifier::new("y", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                    is_pub: false,
                },
            ],
            is_pub: false,
            span: Span::default(),
        };
        compile_struct(&mut mc, &point).unwrap();
        assert!(mc.struct_types.contains_key("Point"));
        let ir = mc.module.print_to_string().to_string();
        assert!(ir.contains("%Point"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_point_struct`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement compile_struct**

```rust
pub fn compile_struct(
    mc: &mut ModuleCompiler,
    struct_def: &StructDef,
) -> Result<(), crate::LlvmError> {
    let name = &struct_def.name.name;

    // Resolve field types
    let mut field_types = Vec::new();
    for field in &struct_def.fields {
        let llvm_ty = crate::types::resolve_type(mc, &field.type_ann)?;
        field_types.push(llvm_ty);
    }

    // Create named struct type
    let struct_type = mc.context.opaque_struct_type(name);
    struct_type.set_body(&field_types, false);

    mc.struct_types.insert(name.clone(), struct_type);
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_point_struct`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/structs.rs
git commit -m "feat(nom-llvm): compile StructDef to LLVM named struct types"
```

---

### Task 5: Implement enum compilation (tagged union)

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/enums.rs`
- Test: inline in `enums.rs`

- [ ] **Step 1: Write failing test**

```rust
use crate::context::ModuleCompiler;
use nom_ast::{EnumDef, EnumVariant, Identifier, Span, TypeExpr};

pub fn compile_enum(
    mc: &mut ModuleCompiler,
    enum_def: &EnumDef,
) -> Result<(), crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    fn make_mc(ctx: &Context) -> ModuleCompiler {
        ModuleCompiler {
            context: ctx,
            module: ctx.create_module("test"),
            builder: ctx.create_builder(),
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn compiles_shape_enum() {
        let ctx = Context::create();
        let mut mc = make_mc(&ctx);
        let shape = EnumDef {
            name: Identifier::new("Shape", Span::default()),
            variants: vec![
                EnumVariant {
                    name: Identifier::new("Circle", Span::default()),
                    fields: vec![TypeExpr::Named(Identifier::new("number", Span::default()))],
                },
                EnumVariant {
                    name: Identifier::new("Rectangle", Span::default()),
                    fields: vec![
                        TypeExpr::Named(Identifier::new("number", Span::default())),
                        TypeExpr::Named(Identifier::new("number", Span::default())),
                    ],
                },
            ],
            is_pub: false,
            span: Span::default(),
        };
        compile_enum(&mut mc, &shape).unwrap();
        assert!(mc.struct_types.contains_key("Shape"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_shape_enum`
Expected: FAIL

- [ ] **Step 3: Implement compile_enum as tagged union**

```rust
pub fn compile_enum(
    mc: &mut ModuleCompiler,
    enum_def: &EnumDef,
) -> Result<(), crate::LlvmError> {
    let name = &enum_def.name.name;
    let i8_type = mc.context.i8_type();

    // Find the maximum payload size across all variants
    let mut max_payload_size: u32 = 0;
    for variant in &enum_def.variants {
        let mut variant_size: u32 = 0;
        for field_ty in &variant.fields {
            let llvm_ty = crate::types::resolve_type(mc, field_ty)?;
            // Approximate size in bytes (8 for f64/i64/ptr, 1 for bool)
            let field_size = match llvm_ty {
                inkwell::types::BasicTypeEnum::FloatType(_) => 8,
                inkwell::types::BasicTypeEnum::IntType(t) => {
                    (t.get_bit_width() as u32 + 7) / 8
                }
                inkwell::types::BasicTypeEnum::PointerType(_) => 8,
                inkwell::types::BasicTypeEnum::StructType(t) => {
                    // Rough estimate: count fields * 8
                    t.count_fields() * 8
                }
                _ => 8,
            };
            variant_size += field_size;
        }
        if variant_size > max_payload_size {
            max_payload_size = variant_size;
        }
    }

    // Enum = { i8 tag, [max_payload_size x i8] payload }
    let payload_type = i8_type.array_type(max_payload_size.max(1));
    let enum_type = mc.context.opaque_struct_type(name);
    enum_type.set_body(&[i8_type.into(), payload_type.into()], false);

    mc.struct_types.insert(name.clone(), enum_type);
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_shape_enum`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/enums.rs
git commit -m "feat(nom-llvm): compile EnumDef to LLVM tagged unions"
```

---

### Task 6: Implement expression compilation

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/expressions.rs`
- Test: inline in `expressions.rs`

- [ ] **Step 1: Write failing test for arithmetic expression**

```rust
use crate::context::ModuleCompiler;
use inkwell::values::BasicValueEnum;
use nom_ast::{BinOp, Expr, Identifier, Literal, Span};

/// Compile a Nom expression to an LLVM value.
pub fn compile_expr<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    expr: &Expr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    fn setup_fn_context(ctx: &Context) -> ModuleCompiler {
        let module = ctx.create_module("test");
        let builder = ctx.create_builder();
        let mut mc = ModuleCompiler {
            context: ctx,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        };

        // Create a dummy function to have a basic block for the builder
        let fn_type = ctx.f64_type().fn_type(&[], false);
        let function = mc.module.add_function("test_fn", fn_type, None);
        let entry = ctx.append_basic_block(function, "entry");
        mc.builder.position_at_end(entry);
        mc
    }

    #[test]
    fn compiles_number_literal() {
        let ctx = Context::create();
        let mut mc = setup_fn_context(&ctx);
        let expr = Expr::Literal(Literal::Number(42.0));
        let val = compile_expr(&mut mc, &expr).unwrap();
        assert!(val.is_float_value());
    }

    #[test]
    fn compiles_addition() {
        let ctx = Context::create();
        let mut mc = setup_fn_context(&ctx);
        let expr = Expr::BinaryOp(
            Box::new(Expr::Literal(Literal::Number(1.0))),
            BinOp::Add,
            Box::new(Expr::Literal(Literal::Number(2.0))),
        );
        let val = compile_expr(&mut mc, &expr).unwrap();
        assert!(val.is_float_value());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_number_literal compiles_addition`
Expected: FAIL

- [ ] **Step 3: Implement expression compilation**

```rust
pub fn compile_expr<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    expr: &Expr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    match expr {
        Expr::Literal(lit) => compile_literal(mc, lit),
        Expr::Ident(ident) => compile_ident(mc, ident),
        Expr::BinaryOp(lhs, op, rhs) => compile_binop(mc, lhs, op, rhs),
        Expr::UnaryOp(op, operand) => compile_unaryop(mc, op, operand),
        Expr::Call(call) => compile_call(mc, call),
        Expr::FieldAccess(obj, field) => compile_field_access(mc, obj, field),
        Expr::IfExpr(if_expr) => crate::statements::compile_if_expr(mc, if_expr),
        Expr::MatchExpr(match_expr) => crate::statements::compile_match_value(mc, match_expr),
        Expr::Array(elements) => compile_array(mc, elements),
        _ => Err(crate::LlvmError::Unsupported(format!("expression: {:?}", std::mem::discriminant(expr)))),
    }
}

fn compile_literal<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    lit: &Literal,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    match lit {
        Literal::Number(n) => Ok(mc.context.f64_type().const_float(*n).into()),
        Literal::Integer(n) => Ok(mc.context.i64_type().const_int(*n as u64, true).into()),
        Literal::Bool(b) => Ok(mc.context.bool_type().const_int(*b as u64, false).into()),
        Literal::Text(s) => {
            // Create a global string constant and return pointer
            let global = mc.builder.build_global_string_ptr(s, "str_lit")
                .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
            Ok(global.as_pointer_value().into())
        }
        Literal::None => Ok(mc.context.i8_type().const_zero().into()),
    }
}

fn compile_ident<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    ident: &Identifier,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    if let Some(ptr) = mc.named_values.get(&ident.name) {
        // Load the value from the alloca
        let val = mc.builder.build_load(mc.context.f64_type(), *ptr, &ident.name)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        Ok(val)
    } else {
        Err(crate::LlvmError::Compilation(format!(
            "undefined variable: {}",
            ident.name
        )))
    }
}

fn compile_binop<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    lhs: &Expr,
    op: &BinOp,
    rhs: &Expr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    let l = compile_expr(mc, lhs)?;
    let r = compile_expr(mc, rhs)?;

    // Float operations (when both sides are f64)
    if l.is_float_value() && r.is_float_value() {
        let lv = l.into_float_value();
        let rv = r.into_float_value();
        let result = match op {
            BinOp::Add => mc.builder.build_float_add(lv, rv, "fadd"),
            BinOp::Sub => mc.builder.build_float_sub(lv, rv, "fsub"),
            BinOp::Mul => mc.builder.build_float_mul(lv, rv, "fmul"),
            BinOp::Div => mc.builder.build_float_div(lv, rv, "fdiv"),
            BinOp::Mod => mc.builder.build_float_rem(lv, rv, "fmod"),
            BinOp::Gt => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OGT, lv, rv, "fcmp_gt"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lt => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OLT, lv, rv, "fcmp_lt"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Gte => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OGE, lv, rv, "fcmp_gte"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lte => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OLE, lv, rv, "fcmp_lte"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Eq => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OEQ, lv, rv, "fcmp_eq"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Neq => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::ONE, lv, rv, "fcmp_neq"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            _ => return Err(crate::LlvmError::Unsupported(format!("binop {:?} on floats", op))),
        }.map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        return Ok(result.into());
    }

    // Integer operations
    if l.is_int_value() && r.is_int_value() {
        let lv = l.into_int_value();
        let rv = r.into_int_value();
        let result = match op {
            BinOp::Add => mc.builder.build_int_add(lv, rv, "iadd"),
            BinOp::Sub => mc.builder.build_int_sub(lv, rv, "isub"),
            BinOp::Mul => mc.builder.build_int_mul(lv, rv, "imul"),
            BinOp::Div => mc.builder.build_int_signed_div(lv, rv, "idiv"),
            BinOp::Mod => mc.builder.build_int_signed_rem(lv, rv, "imod"),
            BinOp::And => mc.builder.build_and(lv, rv, "and"),
            BinOp::Or => mc.builder.build_or(lv, rv, "or"),
            BinOp::BitAnd => mc.builder.build_and(lv, rv, "bitand"),
            BinOp::BitOr => mc.builder.build_or(lv, rv, "bitor"),
            BinOp::Gt => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SGT, lv, rv, "icmp_gt"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lt => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SLT, lv, rv, "icmp_lt"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Eq => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::EQ, lv, rv, "icmp_eq"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Neq => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::NE, lv, rv, "icmp_neq"
                ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            _ => return Err(crate::LlvmError::Unsupported(format!("binop {:?} on integers", op))),
        }.map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        return Ok(result.into());
    }

    Err(crate::LlvmError::Type(format!(
        "type mismatch in binary op {:?}",
        op
    )))
}

fn compile_unaryop<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    op: &nom_ast::UnaryOp,
    operand: &Expr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    let val = compile_expr(mc, operand)?;
    match op {
        nom_ast::UnaryOp::Neg => {
            if val.is_float_value() {
                let r = mc.builder.build_float_neg(val.into_float_value(), "fneg")
                    .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                Ok(r.into())
            } else if val.is_int_value() {
                let r = mc.builder.build_int_neg(val.into_int_value(), "ineg")
                    .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                Ok(r.into())
            } else {
                Err(crate::LlvmError::Type("cannot negate non-numeric type".into()))
            }
        }
        nom_ast::UnaryOp::Not => {
            if val.is_int_value() {
                let r = mc.builder.build_not(val.into_int_value(), "not")
                    .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
                Ok(r.into())
            } else {
                Err(crate::LlvmError::Type("cannot apply ! to non-boolean type".into()))
            }
        }
        _ => Err(crate::LlvmError::Unsupported(format!("unary op {:?}", op))),
    }
}

fn compile_call<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    call: &nom_ast::CallExpr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    let function = mc.functions.get(&call.callee.name)
        .or_else(|| mc.module.get_function(&call.callee.name))
        .ok_or_else(|| crate::LlvmError::Compilation(format!(
            "undefined function: {}", call.callee.name
        )))?;

    let mut args = Vec::new();
    for arg in &call.args {
        let val = compile_expr(mc, arg)?;
        args.push(val.into());
    }

    let call_val = mc.builder
        .build_call(*function, &args, "call")
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;

    // If the function returns void, return a zero i8
    match call_val.try_as_basic_value().left() {
        Some(val) => Ok(val),
        None => Ok(mc.context.i8_type().const_zero().into()),
    }
}

fn compile_field_access<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    obj: &Expr,
    field: &Identifier,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    // Field access on structs will be implemented with GEP
    Err(crate::LlvmError::Unsupported("field access not yet implemented".into()))
}

fn compile_array<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    elements: &[Expr],
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    Err(crate::LlvmError::Unsupported("array literals not yet implemented".into()))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_number_literal compiles_addition`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/expressions.rs
git commit -m "feat(nom-llvm): expression compilation — literals, binops, unaryops, calls"
```

---

### Task 7: Implement statement compilation (let, if, for, while, return)

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/statements.rs`
- Test: inline in `statements.rs`

- [ ] **Step 1: Write failing test for let statement**

```rust
use crate::context::ModuleCompiler;
use inkwell::values::BasicValueEnum;
use nom_ast::*;

/// Compile a block statement to LLVM IR.
pub fn compile_block_stmt<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    stmt: &BlockStmt,
) -> Result<Option<BasicValueEnum<'ctx>>, crate::LlvmError> {
    todo!()
}

/// Compile an if expression (returns a value via phi node).
pub fn compile_if_expr<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    if_expr: &IfExpr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    todo!()
}

/// Compile a match expression (returns a value).
pub fn compile_match_value<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    match_expr: &MatchExpr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    fn setup_fn_context(ctx: &Context) -> (ModuleCompiler, inkwell::values::FunctionValue) {
        let module = ctx.create_module("test");
        let builder = ctx.create_builder();
        let mut mc = ModuleCompiler {
            context: ctx,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        };
        let fn_type = ctx.f64_type().fn_type(&[], false);
        let function = mc.module.add_function("test_fn", fn_type, None);
        let entry = ctx.append_basic_block(function, "entry");
        mc.builder.position_at_end(entry);
        (mc, function)
    }

    #[test]
    fn compiles_let_stmt() {
        let ctx = Context::create();
        let (mut mc, _) = setup_fn_context(&ctx);
        let stmt = BlockStmt::Let(LetStmt {
            name: Identifier::new("x", Span::default()),
            mutable: false,
            type_ann: Some(TypeExpr::Named(Identifier::new("number", Span::default()))),
            value: Expr::Literal(Literal::Number(42.0)),
            span: Span::default(),
        });
        let result = compile_block_stmt(&mut mc, &stmt).unwrap();
        assert!(mc.named_values.contains_key("x"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_let_stmt`
Expected: FAIL

- [ ] **Step 3: Implement statement compilation**

```rust
pub fn compile_block_stmt<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    stmt: &BlockStmt,
) -> Result<Option<BasicValueEnum<'ctx>>, crate::LlvmError> {
    match stmt {
        BlockStmt::Let(let_stmt) => {
            compile_let(mc, let_stmt)?;
            Ok(None)
        }
        BlockStmt::Assign(assign) => {
            compile_assign(mc, assign)?;
            Ok(None)
        }
        BlockStmt::Expr(expr) => {
            let val = crate::expressions::compile_expr(mc, expr)?;
            Ok(Some(val))
        }
        BlockStmt::If(if_expr) => {
            let val = compile_if_expr(mc, if_expr)?;
            Ok(Some(val))
        }
        BlockStmt::For(for_stmt) => {
            compile_for(mc, for_stmt)?;
            Ok(None)
        }
        BlockStmt::While(while_stmt) => {
            compile_while(mc, while_stmt)?;
            Ok(None)
        }
        BlockStmt::Return(expr) => {
            if let Some(e) = expr {
                let val = crate::expressions::compile_expr(mc, e)?;
                mc.builder.build_return(Some(&val))
                    .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
            } else {
                mc.builder.build_return(None)
                    .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
            }
            Ok(None)
        }
        BlockStmt::Break => {
            // Break is handled in loop context
            Ok(None)
        }
        BlockStmt::Continue => {
            // Continue is handled in loop context
            Ok(None)
        }
        BlockStmt::Match(match_expr) => {
            let val = compile_match_value(mc, match_expr)?;
            Ok(Some(val))
        }
    }
}

fn compile_let<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    let_stmt: &LetStmt,
) -> Result<(), crate::LlvmError> {
    let val = crate::expressions::compile_expr(mc, &let_stmt.value)?;

    // Determine type for alloca
    let alloca_type = if let Some(ref type_ann) = let_stmt.type_ann {
        crate::types::resolve_type(mc, type_ann)?
    } else {
        // Infer from value
        match val {
            BasicValueEnum::FloatValue(_) => mc.context.f64_type().into(),
            BasicValueEnum::IntValue(iv) => iv.get_type().into(),
            BasicValueEnum::PointerValue(_) => mc.context.ptr_type(inkwell::AddressSpace::default()).into(),
            _ => mc.context.f64_type().into(),
        }
    };

    // Create alloca at function entry
    let alloca = mc.builder.build_alloca(alloca_type, &let_stmt.name.name)
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
    mc.builder.build_store(alloca, val)
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;

    mc.named_values.insert(let_stmt.name.name.clone(), alloca);
    Ok(())
}

fn compile_assign<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    assign: &AssignStmt,
) -> Result<(), crate::LlvmError> {
    let val = crate::expressions::compile_expr(mc, &assign.value)?;

    if let Expr::Ident(ref ident) = assign.target {
        let ptr = mc.named_values.get(&ident.name)
            .ok_or_else(|| crate::LlvmError::Compilation(format!(
                "undefined variable for assignment: {}", ident.name
            )))?;
        mc.builder.build_store(*ptr, val)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        Ok(())
    } else {
        Err(crate::LlvmError::Unsupported("complex assignment targets not yet supported".into()))
    }
}

pub fn compile_if_expr<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    if_expr: &IfExpr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    let cond_val = crate::expressions::compile_expr(mc, &if_expr.condition)?;

    // Convert condition to i1 if needed
    let cond_bool = if cond_val.is_int_value() {
        cond_val.into_int_value()
    } else if cond_val.is_float_value() {
        // Float != 0.0
        mc.builder.build_float_compare(
            inkwell::FloatPredicate::ONE,
            cond_val.into_float_value(),
            mc.context.f64_type().const_float(0.0),
            "cond_bool",
        ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?
    } else {
        return Err(crate::LlvmError::Type("if condition must be boolean or numeric".into()));
    };

    let function = mc.builder.get_insert_block().unwrap().get_parent().unwrap();
    let then_bb = mc.context.append_basic_block(function, "then");
    let else_bb = mc.context.append_basic_block(function, "else");
    let merge_bb = mc.context.append_basic_block(function, "ifmerge");

    mc.builder.build_conditional_branch(cond_bool, then_bb, else_bb)
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;

    // Then block
    mc.builder.position_at_end(then_bb);
    let mut then_val = mc.context.f64_type().const_float(0.0).into();
    for stmt in &if_expr.then_body.stmts {
        if let Some(val) = compile_block_stmt(mc, stmt)? {
            then_val = val;
        }
    }
    // Only branch to merge if current block has no terminator
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder.build_unconditional_branch(merge_bb)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
    }
    let then_end_bb = mc.builder.get_insert_block().unwrap();

    // Else block
    mc.builder.position_at_end(else_bb);
    let mut else_val = mc.context.f64_type().const_float(0.0).into();
    if let Some(ref else_body) = if_expr.else_body {
        for stmt in &else_body.stmts {
            if let Some(val) = compile_block_stmt(mc, stmt)? {
                else_val = val;
            }
        }
    }
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder.build_unconditional_branch(merge_bb)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
    }
    let else_end_bb = mc.builder.get_insert_block().unwrap();

    // Merge block
    mc.builder.position_at_end(merge_bb);

    // Return a default value (phi nodes for proper value-returning if would be more complex)
    Ok(mc.context.f64_type().const_float(0.0).into())
}

fn compile_for<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    for_stmt: &ForStmt,
) -> Result<(), crate::LlvmError> {
    // For now, compile for loops over integer ranges: for i in 0..n
    // General iterable support will come later
    Err(crate::LlvmError::Unsupported("for loops not yet fully implemented — use while loops".into()))
}

fn compile_while<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    while_stmt: &WhileStmt,
) -> Result<(), crate::LlvmError> {
    let function = mc.builder.get_insert_block().unwrap().get_parent().unwrap();
    let cond_bb = mc.context.append_basic_block(function, "while_cond");
    let body_bb = mc.context.append_basic_block(function, "while_body");
    let end_bb = mc.context.append_basic_block(function, "while_end");

    mc.builder.build_unconditional_branch(cond_bb)
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;

    // Condition
    mc.builder.position_at_end(cond_bb);
    let cond_val = crate::expressions::compile_expr(mc, &while_stmt.condition)?;
    let cond_bool = if cond_val.is_int_value() {
        cond_val.into_int_value()
    } else {
        mc.builder.build_float_compare(
            inkwell::FloatPredicate::ONE,
            cond_val.into_float_value(),
            mc.context.f64_type().const_float(0.0),
            "while_cond_bool",
        ).map_err(|e| crate::LlvmError::Compilation(e.to_string()))?
    };
    mc.builder.build_conditional_branch(cond_bool, body_bb, end_bb)
        .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;

    // Body
    mc.builder.position_at_end(body_bb);
    for stmt in &while_stmt.body.stmts {
        compile_block_stmt(mc, stmt)?;
    }
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder.build_unconditional_branch(cond_bb)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
    }

    mc.builder.position_at_end(end_bb);
    Ok(())
}

pub fn compile_match_value<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    _match_expr: &MatchExpr,
) -> Result<BasicValueEnum<'ctx>, crate::LlvmError> {
    // Match compilation will be implemented with switch instruction
    Err(crate::LlvmError::Unsupported("match expressions not yet implemented for LLVM".into()))
}

/// Compile a full Block (used in function bodies).
pub fn compile_block<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    block: &Block,
) -> Result<Option<BasicValueEnum<'ctx>>, crate::LlvmError> {
    let mut last_val = None;
    for stmt in &block.stmts {
        last_val = compile_block_stmt(mc, stmt)?;
    }
    Ok(last_val)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_let_stmt`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/statements.rs
git commit -m "feat(nom-llvm): statement compilation — let, assign, if, while, return"
```

---

### Task 8: Implement function compilation

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/functions.rs`
- Test: inline in `functions.rs`

- [ ] **Step 1: Write failing test for function compilation**

```rust
use crate::context::ModuleCompiler;
use nom_ast::*;

pub fn compile_fn(
    mc: &mut ModuleCompiler,
    fn_def: &FnDef,
) -> Result<(), crate::LlvmError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    fn make_mc(ctx: &Context) -> ModuleCompiler {
        ModuleCompiler {
            context: ctx,
            module: ctx.create_module("test"),
            builder: ctx.create_builder(),
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn compiles_add_function() {
        let ctx = Context::create();
        let mut mc = make_mc(&ctx);
        let fn_def = FnDef {
            name: Identifier::new("add", Span::default()),
            params: vec![
                FnParam {
                    name: Identifier::new("a", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                },
                FnParam {
                    name: Identifier::new("b", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                },
            ],
            return_type: Some(TypeExpr::Named(Identifier::new("number", Span::default()))),
            body: Block {
                stmts: vec![BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::Ident(Identifier::new("a", Span::default()))),
                    BinOp::Add,
                    Box::new(Expr::Ident(Identifier::new("b", Span::default()))),
                )))],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };
        compile_fn(&mut mc, &fn_def).unwrap();
        let ir = mc.module.print_to_string().to_string();
        assert!(ir.contains("define double @add(double %a, double %b)"));
        assert!(ir.contains("fadd"));
        assert!(ir.contains("ret double"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_add_function`
Expected: FAIL

- [ ] **Step 3: Implement function compilation**

```rust
pub fn compile_fn(
    mc: &mut ModuleCompiler,
    fn_def: &FnDef,
) -> Result<(), crate::LlvmError> {
    // Resolve parameter types
    let mut param_types = Vec::new();
    for param in &fn_def.params {
        let llvm_ty = crate::types::resolve_type(mc, &param.type_ann)?;
        param_types.push(llvm_ty.into());
    }

    // Resolve return type
    let fn_type = if let Some(ref ret_type) = fn_def.return_type {
        let ret_llvm = crate::types::resolve_type(mc, ret_type)?;
        match ret_llvm {
            inkwell::types::BasicTypeEnum::FloatType(ft) => ft.fn_type(&param_types, false),
            inkwell::types::BasicTypeEnum::IntType(it) => it.fn_type(&param_types, false),
            inkwell::types::BasicTypeEnum::PointerType(pt) => pt.fn_type(&param_types, false),
            inkwell::types::BasicTypeEnum::StructType(st) => st.fn_type(&param_types, false),
            _ => mc.context.void_type().fn_type(&param_types, false),
        }
    } else {
        mc.context.void_type().fn_type(&param_types, false)
    };

    // Create function
    let function = mc.module.add_function(&fn_def.name.name, fn_type, None);
    mc.functions.insert(fn_def.name.name.clone(), function);

    // Create entry basic block
    let entry = mc.context.append_basic_block(function, "entry");
    mc.builder.position_at_end(entry);

    // Save outer scope and create new scope for function body
    let outer_values = mc.named_values.clone();
    mc.named_values.clear();

    // Bind parameters: create allocas and store param values
    for (i, param) in fn_def.params.iter().enumerate() {
        let param_val = function.get_nth_param(i as u32).unwrap();
        param_val.set_name(&param.name.name);

        let llvm_ty = crate::types::resolve_type(mc, &param.type_ann)?;
        let alloca = mc.builder.build_alloca(llvm_ty, &param.name.name)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        mc.builder.build_store(alloca, param_val)
            .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        mc.named_values.insert(param.name.name.clone(), alloca);
    }

    // Compile function body
    crate::statements::compile_block(mc, &fn_def.body)?;

    // If no explicit return, add one
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        if fn_def.return_type.is_none() {
            mc.builder.build_return(None)
                .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        } else {
            // Return default value
            let ret_ty = crate::types::resolve_type(mc, fn_def.return_type.as_ref().unwrap())?;
            let default_val = match ret_ty {
                inkwell::types::BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                inkwell::types::BasicTypeEnum::IntType(it) => it.const_zero().into(),
                _ => mc.context.f64_type().const_float(0.0).into(),
            };
            mc.builder.build_return(Some(&default_val))
                .map_err(|e| crate::LlvmError::Compilation(e.to_string()))?;
        }
    }

    // Verify function
    if !function.verify(true) {
        return Err(crate::LlvmError::Verification(format!(
            "function '{}' failed LLVM verification",
            fn_def.name.name
        )));
    }

    // Restore outer scope
    mc.named_values = outer_values;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compiles_add_function`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/functions.rs
git commit -m "feat(nom-llvm): function compilation with params, return types, body"
```

---

### Task 9: End-to-end test — compile imperative.nom to LLVM IR

**Files:**
- Modify: `nom-compiler/crates/nom-llvm/src/lib.rs` (add integration test)

- [ ] **Step 1: Write end-to-end test**

Add to `nom-compiler/crates/nom-llvm/src/lib.rs`:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use nom_parser::parse_source;
    use nom_planner::{CompositionPlan, FlowPlan, MemoryStrategy, ConcurrencyStrategy};

    #[test]
    fn compile_geometry_program() {
        // Build a plan that contains imperative stmts from imperative.nom
        let source = r#"
nom geometry
  struct Point {
    x: number,
    y: number
  }
  fn add(a: number, b: number) -> number {
    return a + b
  }
"#;
        let sf = parse_source(source).expect("parse failed");

        // Extract imperative statements from parsed source
        let mut imperative_stmts = Vec::new();
        for decl in &sf.declarations {
            for stmt in &decl.statements {
                match stmt {
                    nom_ast::Statement::StructDef(_)
                    | nom_ast::Statement::FnDef(_)
                    | nom_ast::Statement::EnumDef(_) => {
                        imperative_stmts.push(stmt.clone());
                    }
                    _ => {}
                }
            }
        }

        let plan = CompositionPlan {
            source_path: Some("geometry.nom".into()),
            flows: vec![FlowPlan {
                name: "geometry".into(),
                classifier: "nom".into(),
                agent: None,
                graph: None,
                nodes: vec![],
                edges: vec![],
                branches: vec![],
                memory_strategy: MemoryStrategy::Stack,
                concurrency_strategy: ConcurrencyStrategy::Sequential,
                effect_summary: vec![],
                imperative_stmts,
            }],
            nomiz: "{}".into(),
        };

        let output = compile(&plan).expect("LLVM compilation failed");
        let ir = &output.ir_text;

        // Verify struct and function appear in IR
        assert!(ir.contains("%Point"), "Point struct not in IR:\n{}", ir);
        assert!(ir.contains("@add"), "add function not in IR:\n{}", ir);
        assert!(ir.contains("fadd"), "fadd not in IR:\n{}", ir);
        assert!(!output.bitcode.is_empty(), "bitcode should not be empty");
    }
}
```

- [ ] **Step 2: Run test**

Run: `cd nom-compiler && cargo test -p nom-llvm -- compile_geometry_program`
Expected: PASS if all prior tasks are correct. If it fails, debug by checking which specific assertion fails.

- [ ] **Step 3: Commit**

```bash
git add nom-compiler/crates/nom-llvm/src/lib.rs
git commit -m "test(nom-llvm): end-to-end test — compile geometry struct + fn to LLVM IR"
```

---

### Task 10: Wire nom-llvm into the CLI

**Files:**
- Modify: `nom-compiler/crates/nom-cli/Cargo.toml`
- Modify: `nom-compiler/crates/nom-cli/src/main.rs`

- [ ] **Step 1: Add nom-llvm dependency to CLI**

In `nom-compiler/crates/nom-cli/Cargo.toml`, add:
```toml
nom-llvm = { path = "../nom-llvm" }
```

- [ ] **Step 2: Add --target flag to build command**

In `nom-compiler/crates/nom-cli/src/main.rs`, modify the `Build` variant in `Commands` enum to add:
```rust
    /// Compilation target: rust (default), llvm, native
    #[arg(long, default_value = "rust")]
    target: String,
```

- [ ] **Step 3: Add LLVM build path in cmd_build function**

Find the `cmd_build` function and add an LLVM target branch. When `--target llvm` is specified:
1. Parse the .nom file
2. Plan it (same as current)
3. Call `nom_llvm::compile(&plan)` instead of `nom_codegen::generate`
4. Write the `.bc` file to output path
5. Optionally invoke `llc` + `lld` for `--target native`

```rust
// Add this import at the top of main.rs:
// use nom_llvm;

// In the build command handler, after creating the plan:
if target == "llvm" || target == "native" {
    let output = nom_llvm::compile(&plan)
        .map_err(|e| format!("LLVM compilation failed: {}", e))?;

    // Write .bc file
    let bc_path = out_path.with_extension("bc");
    std::fs::write(&bc_path, &output.bitcode)
        .map_err(|e| format!("failed to write bitcode: {}", e))?;
    println!("  wrote {}", bc_path.display());

    // Write .ll file for debugging
    let ll_path = out_path.with_extension("ll");
    std::fs::write(&ll_path, &output.ir_text)
        .map_err(|e| format!("failed to write IR: {}", e))?;
    println!("  wrote {}", ll_path.display());

    if target == "native" {
        // Invoke llc to compile .bc → .o
        let obj_path = out_path.with_extension("o");
        let llc_status = std::process::Command::new("llc")
            .args(&["-filetype=obj", "-o"])
            .arg(&obj_path)
            .arg(&bc_path)
            .status();
        match llc_status {
            Ok(status) if status.success() => {
                println!("  compiled to {}", obj_path.display());
            }
            _ => {
                eprintln!("  warning: llc not found or failed — install LLVM tools for native compilation");
                eprintln!("  bitcode written to {} — use `llc` manually", bc_path.display());
            }
        }
    }
} else {
    // Existing Rust codegen path
}
```

- [ ] **Step 4: Test the CLI flag compiles**

Run: `cd nom-compiler && cargo check -p nom-cli`
Expected: Compiles successfully.

- [ ] **Step 5: Test with a .nom file (manual)**

Run: `cd nom-compiler && cargo run -- build --target llvm examples/imperative.nom`
Expected: Produces `imperative.bc` and `imperative.ll` files (or a clear error about what's not yet supported).

- [ ] **Step 6: Commit**

```bash
git add nom-compiler/crates/nom-cli/Cargo.toml nom-compiler/crates/nom-cli/src/main.rs
git commit -m "feat(nom-cli): add --target llvm/native flag to nom build command"
```

---

### Task 11: Run all existing tests to verify no regressions

**Files:** None modified — verification only.

- [ ] **Step 1: Run full test suite**

Run: `cd nom-compiler && cargo test --workspace`
Expected: All 114+ existing tests pass. New nom-llvm tests also pass.

- [ ] **Step 2: If any failures, fix them**

Common issues:
- nom-codegen `Target` enum may need updating if shared code changed
- Import paths may need adjusting

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix: resolve test regressions from nom-llvm integration"
```

---

## Summary

| Task | What it builds | Key test |
|------|---------------|----------|
| 1 | Crate scaffold + inkwell | `cargo check -p nom-llvm` |
| 2 | NomCompiler context + runtime | `empty_plan_produces_valid_ir` |
| 3 | Type mapping | `number_maps_to_f64`, `bool_maps_to_i1` |
| 4 | Struct compilation | `compiles_point_struct` |
| 5 | Enum compilation | `compiles_shape_enum` |
| 6 | Expression compilation | `compiles_number_literal`, `compiles_addition` |
| 7 | Statement compilation | `compiles_let_stmt` |
| 8 | Function compilation | `compiles_add_function` |
| 9 | End-to-end integration | `compile_geometry_program` |
| 10 | CLI integration | `cargo run -- build --target llvm` |
| 11 | Regression check | `cargo test --workspace` |
