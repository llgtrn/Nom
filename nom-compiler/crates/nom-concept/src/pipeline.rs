// Full compilation pipeline: source → AST → IR → native stub

#[derive(Debug, Clone)]
pub struct CompileInput {
    pub source: String,
    pub entry_name: String,
}

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub entry_name: String,
    pub ir_text: String,
    pub binary_bytes: Vec<u8>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileStage {
    Parse,
    TypeCheck,
    IrLower,
    Codegen,
    Done,
}

#[derive(Debug)]
pub struct CompileError {
    pub stage: CompileStage,
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.stage, self.message)
    }
}

/// Drives the full pipeline from source to binary stub.
pub struct CompilePipeline {
    pub strict_mode: bool,
}

impl CompilePipeline {
    pub fn new() -> Self { Self { strict_mode: false } }
    pub fn strict() -> Self { Self { strict_mode: true } }

    pub fn compile(&self, input: CompileInput) -> Result<CompileOutput, CompileError> {
        // Stage 1: basic parse check
        if input.source.trim().is_empty() {
            return Err(CompileError { stage: CompileStage::Parse, message: "empty source".into() });
        }
        // Stage 2: IR lowering (stub — produce human-readable IR text)
        let ir_text = format!("; IR for {}\ndefine {} void () {{\n  ret void\n}}", input.entry_name, input.entry_name);
        // Stage 3: codegen stub — 0xC3 RET per function
        let binary_bytes = vec![0xC3u8]; // single RET instruction stub
        Ok(CompileOutput {
            entry_name: input.entry_name,
            ir_text,
            binary_bytes,
            warnings: vec![],
        })
    }

    pub fn stage_names() -> &'static [&'static str] {
        &["parse", "typecheck", "ir_lower", "codegen", "done"]
    }
}

impl Default for CompilePipeline {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    #[test]
    fn test_compile_basic() {
        let pipeline = CompilePipeline::new();
        let input = CompileInput {
            source: "define greet that \"hello\"".into(),
            entry_name: "greet".into(),
        };
        let result = pipeline.compile(input);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.entry_name, "greet");
        assert!(!out.ir_text.is_empty());
        assert!(!out.binary_bytes.is_empty());
    }

    #[test]
    fn test_compile_empty_source() {
        let pipeline = CompilePipeline::new();
        let input = CompileInput { source: "".into(), entry_name: "x".into() };
        let result = pipeline.compile(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.stage, CompileStage::Parse);
    }

    #[test]
    fn test_compile_produces_ret_stub() {
        let pipeline = CompilePipeline::new();
        let input = CompileInput { source: "define x that 1".into(), entry_name: "x".into() };
        let out = pipeline.compile(input).unwrap();
        assert_eq!(out.binary_bytes, vec![0xC3]);
    }

    #[test]
    fn test_compile_ir_contains_entry_name() {
        let pipeline = CompilePipeline::new();
        let input = CompileInput { source: "define foo that bar".into(), entry_name: "foo".into() };
        let out = pipeline.compile(input).unwrap();
        assert!(out.ir_text.contains("foo"));
    }

    #[test]
    fn test_strict_pipeline() {
        let pipeline = CompilePipeline::strict();
        assert!(pipeline.strict_mode);
    }

    #[test]
    fn test_stage_names() {
        let names = CompilePipeline::stage_names();
        assert_eq!(names.len(), 5);
        assert_eq!(names[0], "parse");
        assert_eq!(names[4], "done");
    }

    #[test]
    fn test_compile_no_warnings_for_valid() {
        let pipeline = CompilePipeline::new();
        let input = CompileInput { source: "define foo that 42".into(), entry_name: "foo".into() };
        let out = pipeline.compile(input).unwrap();
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn test_compile_error_display() {
        let err = CompileError { stage: CompileStage::TypeCheck, message: "type mismatch".into() };
        let s = format!("{}", err);
        assert!(s.contains("type mismatch"));
    }
}
