//! Parser stub and AST → IR codegen for the `.nomx` source format.
//!
//! This module bridges raw `.nomx` source text to the typed IR defined in
//! [`crate::ir`].  Three types carry the pipeline:
//!
//! 1. [`NomAst`] / [`NomDef`] — lightweight parsed representation of
//!    `define X that Y` declarations.
//! 2. [`AstToIr`] — lowers an [`NomAst`] to an [`IrModule`].
//! 3. [`IrPrinter`] — produces a human-readable text dump of an [`IrModule`].

use crate::ir::{IrFunction, IrInstr, IrModule, IrType, IrValue};

// ─── AST ────────────────────────────────────────────────────────────────────

/// A parsed `.nomx` source file containing zero or more definitions.
#[derive(Debug, Default, Clone)]
pub struct NomAst {
    pub definitions: Vec<NomDef>,
}

/// A single `define <name> that <body>` declaration.
#[derive(Debug, Clone)]
pub struct NomDef {
    pub name: String,
    /// Whitespace-separated parameter tokens following the name (before `that`).
    pub params: Vec<String>,
    /// Everything after the `that` keyword on the same line, trimmed.
    pub body: String,
}

impl NomAst {
    /// Parse a `.nomx` source string.
    ///
    /// Each non-empty, non-comment line is examined for the pattern
    /// `define <name> [params…] that <body>`.  Lines that do not match this
    /// pattern are silently skipped (stub behaviour).  Returns an error if the
    /// source contains a `define` line that is structurally malformed (i.e.
    /// has `define` but is missing `that`).
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut definitions = Vec::new();

        for (line_idx, raw) in source.lines().enumerate() {
            let line = raw.trim();

            // Skip blank lines and `//` comments.
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Only process lines that start with `define`.
            if !line.starts_with("define ") {
                continue;
            }

            // Validate that `that` separator is present.
            let Some(that_pos) = line.find(" that ") else {
                return Err(format!(
                    "line {}: `define` line is missing `that` separator: `{line}`",
                    line_idx + 1
                ));
            };

            // Tokens between `define ` and ` that `.
            let between = line["define ".len()..that_pos].trim();
            let mut tokens = between.split_whitespace();
            let name = tokens
                .next()
                .ok_or_else(|| {
                    format!("line {}: `define` has no name before `that`", line_idx + 1)
                })?
                .to_owned();
            let params: Vec<String> = tokens.map(str::to_owned).collect();

            let body = line[that_pos + " that ".len()..].trim().to_owned();

            definitions.push(NomDef { name, params, body });
        }

        Ok(NomAst { definitions })
    }

    /// Number of top-level definitions in the AST.
    pub fn definition_count(&self) -> usize {
        self.definitions.len()
    }

    /// Look up a definition by name.
    pub fn find_def(&self, name: &str) -> Option<&NomDef> {
        self.definitions.iter().find(|d| d.name == name)
    }
}

// ─── AST → IR lowering ──────────────────────────────────────────────────────

/// Lowers a [`NomAst`] to an [`IrModule`].
pub struct AstToIr;

impl AstToIr {
    /// Lower every definition in `ast` into a function in a new [`IrModule`].
    ///
    /// The resulting module is named `"nom_module"`.  Each [`NomDef`] becomes
    /// an [`IrFunction`] with a single [`IrInstr::Return`] whose value is the
    /// definition's body text.
    pub fn lower(ast: &NomAst) -> IrModule {
        let mut module = IrModule::new("nom_module");
        for def in &ast.definitions {
            module.functions.push(Self::lower_def(def));
        }
        module
    }

    /// Lower a single [`NomDef`] to an [`IrFunction`].
    ///
    /// Parameters become `IrType::Str` parameters on the function.  The body
    /// is returned as a string constant.
    pub fn lower_def(def: &NomDef) -> IrFunction {
        let mut func = IrFunction::new(&def.name, IrType::Str);
        for param in &def.params {
            func = func.with_param(param, IrType::Str);
        }
        func = func.push_instr(IrInstr::Return(IrValue::Str(def.body.clone())));
        func
    }
}

// ─── IR Printer ─────────────────────────────────────────────────────────────

/// Produces a human-readable text dump of an [`IrModule`].
pub struct IrPrinter;

impl IrPrinter {
    /// Render an entire [`IrModule`] as a multi-line string.
    ///
    /// Format:
    /// ```text
    /// module <name>
    /// fn <name>: <instr> …
    /// ```
    pub fn print_module(module: &IrModule) -> String {
        let mut out = format!("module {}", module.name);
        for func in &module.functions {
            out.push('\n');
            out.push_str(&Self::print_function(func));
        }
        out
    }

    /// Render a single [`IrFunction`] as one line.
    ///
    /// Format: `fn <name>: <instr0>, <instr1>, …`
    pub fn print_function(func: &IrFunction) -> String {
        let instrs: Vec<String> = func.body.iter().map(Self::print_instr).collect();
        format!("fn {}: {}", func.name, instrs.join(", "))
    }

    fn print_instr(instr: &IrInstr) -> String {
        match instr {
            IrInstr::Assign { dest, value } => {
                format!("{dest} = {}", Self::print_value(value))
            }
            IrInstr::Call { func, args, dest } => {
                let args_str: Vec<String> = args.iter().map(Self::print_value).collect();
                let lhs = if let Some(d) = dest {
                    format!("{d} = ")
                } else {
                    String::new()
                };
                format!("{}call {}({})", lhs, func, args_str.join(", "))
            }
            IrInstr::Return(v) => format!("return {}", Self::print_value(v)),
            IrInstr::BranchIf {
                cond,
                then_label,
                else_label,
            } => {
                format!("branch_if {cond} then:{then_label} else:{else_label}")
            }
            IrInstr::Label(l) => format!("label:{l}"),
            IrInstr::Nop => "nop".to_owned(),
        }
    }

    fn print_value(value: &IrValue) -> String {
        match value {
            IrValue::Unit => "()".to_owned(),
            IrValue::Bool(b) => b.to_string(),
            IrValue::Int(i) => i.to_string(),
            IrValue::Float(f) => format!("{f}"),
            IrValue::Str(s) => format!("\"{s}\""),
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom_ast_parse_empty_source() {
        let ast = NomAst::parse("").unwrap();
        assert_eq!(ast.definition_count(), 0);
    }

    #[test]
    fn nom_ast_parse_single_define_that() {
        let src = "define greet that hello world";
        let ast = NomAst::parse(src).unwrap();
        assert_eq!(ast.definition_count(), 1);
        let def = &ast.definitions[0];
        assert_eq!(def.name, "greet");
        assert!(def.params.is_empty());
        assert_eq!(def.body, "hello world");
    }

    #[test]
    fn nom_ast_parse_multiple_defs() {
        let src = "define foo that bar\ndefine baz that qux\n";
        let ast = NomAst::parse(src).unwrap();
        assert_eq!(ast.definition_count(), 2);
        assert_eq!(ast.definitions[0].name, "foo");
        assert_eq!(ast.definitions[1].name, "baz");
    }

    #[test]
    fn nom_ast_find_def() {
        let src = "define alpha that one\ndefine beta that two";
        let ast = NomAst::parse(src).unwrap();
        assert!(ast.find_def("alpha").is_some());
        assert_eq!(ast.find_def("alpha").unwrap().body, "one");
        assert!(ast.find_def("gamma").is_none());
    }

    #[test]
    fn ast_to_ir_lower_empty() {
        let ast = NomAst::default();
        let module = AstToIr::lower(&ast);
        assert_eq!(module.name, "nom_module");
        assert_eq!(module.function_count(), 0);
    }

    #[test]
    fn ast_to_ir_lower_single_def() {
        let src = "define compute that result";
        let ast = NomAst::parse(src).unwrap();
        let module = AstToIr::lower(&ast);
        assert_eq!(module.function_count(), 1);
        let func = module.get_function("compute").unwrap();
        assert!(func.has_return());
        assert_eq!(func.instr_count(), 1);
        if let crate::ir::IrInstr::Return(IrValue::Str(body)) = &func.body[0] {
            assert_eq!(body, "result");
        } else {
            panic!("expected Return(Str(...))");
        }
    }

    #[test]
    fn ir_printer_print_empty_module() {
        let module = IrModule::new("empty");
        let text = IrPrinter::print_module(&module);
        assert_eq!(text, "module empty");
    }

    #[test]
    fn ir_printer_print_function() {
        let func = IrFunction::new("greet", IrType::Str)
            .push_instr(IrInstr::Return(IrValue::Str("hello".into())));
        let text = IrPrinter::print_function(&func);
        assert_eq!(text, "fn greet: return \"hello\"");
    }
}
