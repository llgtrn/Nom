use std::collections::HashMap;

/// A user prompt for code generation.
#[derive(Debug, Clone)]
pub struct CodePrompt {
    pub text: String,
    pub context: Option<String>,
    pub entrypoint_hint: Option<String>,
}

impl CodePrompt {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), context: None, entrypoint_hint: None }
    }

    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }
}

/// System prompt templates (gpt-engineer PrepromptHolder pattern).
#[derive(Debug, Clone)]
pub struct PrepromptHolder {
    pub roadmap: String,
    pub generate: String,
    pub improve: String,
    pub file_format: String,
}

impl Default for PrepromptHolder {
    fn default() -> Self {
        Self {
            roadmap: "Think step by step. Reason through the problem.".into(),
            generate: "Output all files needed. Use the file format below.".into(),
            improve: "Improve the existing code. Keep what works, fix what doesn't.".into(),
            file_format: "filepath\n```lang\ncode\n```".into(),
        }
    }
}

impl PrepromptHolder {
    pub fn build_system_prompt(&self, mode: &GenerationMode) -> String {
        match mode {
            GenerationMode::Generate => format!("{}\n\n{}\n\n{}", self.roadmap, self.generate, self.file_format),
            GenerationMode::Improve => format!("{}\n\n{}\n\n{}", self.roadmap, self.improve, self.file_format),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenerationMode {
    Generate,
    Improve,
}

/// A collection of generated files (path → content).
#[derive(Debug, Clone, Default)]
pub struct FilesDict {
    pub files: HashMap<String, String>,
}

impl FilesDict {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    pub fn file_count(&self) -> usize { self.files.len() }

    pub fn has_entrypoint(&self) -> bool {
        self.files.contains_key("run.sh") || self.files.contains_key("main.rs")
            || self.files.contains_key("main.py") || self.files.contains_key("index.js")
    }

    /// Parse LLM output in gpt-engineer file format into FilesDict.
    pub fn from_llm_output(output: &str) -> Self {
        let mut dict = Self::new();
        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            // Look for filepath pattern: line with no spaces that ends in known extension
            if !line.is_empty() && !line.starts_with("```") && (
                line.ends_with(".rs") || line.ends_with(".py") || line.ends_with(".js")
                || line.ends_with(".ts") || line.ends_with(".html") || line.ends_with(".sh")
            ) {
                let path = line.to_string();
                i += 1;
                // Next line might be ``` fence
                if i < lines.len() && lines[i].trim().starts_with("```") { i += 1; }
                let mut content_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    content_lines.push(lines[i]);
                    i += 1;
                }
                if i < lines.len() { i += 1; } // skip closing ```
                dict.insert(path, content_lines.join("\n"));
            } else {
                i += 1;
            }
        }
        dict
    }
}

/// One iteration of the code generation loop.
#[derive(Debug, Clone)]
pub struct GenerationStep {
    pub step: usize,
    pub mode: GenerationMode,
    pub files: FilesDict,
    pub entrypoint: Option<String>,
}

/// Code generation pipeline (gpt-engineer pattern).
pub struct CodeGenPipeline {
    pub prompts: PrepromptHolder,
    pub max_iterations: usize,
}

impl CodeGenPipeline {
    pub fn new() -> Self {
        Self { prompts: PrepromptHolder::default(), max_iterations: 3 }
    }

    /// Generate code from a prompt (stub — returns synthetic FilesDict).
    pub fn generate(&self, prompt: &CodePrompt) -> GenerationStep {
        let mut files = FilesDict::new();
        // Stub: generate based on prompt text
        let lang = if prompt.text.contains("rust") { "rs" }
            else if prompt.text.contains("python") { "py" }
            else { "js" };
        files.insert(format!("main.{}", lang), format!("// Generated from: {}", prompt.text));
        files.insert("run.sh".to_string(), "#!/bin/sh\necho 'running'".to_string());
        let system_prompt = self.prompts.build_system_prompt(&GenerationMode::Generate);
        GenerationStep {
            step: 1,
            mode: GenerationMode::Generate,
            files,
            entrypoint: Some(format!("run.sh (system: {} chars)", system_prompt.len())),
        }
    }

    /// Improve existing code (stub — adds an improved comment).
    pub fn improve(&self, existing: &mut FilesDict, feedback: &str) -> GenerationStep {
        for (_, content) in existing.files.iter_mut() {
            *content = format!("// Improved: {}\n{}", feedback, content);
        }
        GenerationStep {
            step: 2,
            mode: GenerationMode::Improve,
            files: existing.clone(),
            entrypoint: None,
        }
    }
}

impl Default for CodeGenPipeline {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod codegen_tests {
    use super::*;

    #[test]
    fn test_code_prompt_new() {
        let p = CodePrompt::new("build a todo app");
        assert_eq!(p.text, "build a todo app");
        assert!(p.context.is_none());
    }

    #[test]
    fn test_files_dict_insert_get() {
        let mut d = FilesDict::new();
        d.insert("main.rs", "fn main() {}");
        assert_eq!(d.get("main.rs"), Some("fn main() {}"));
        assert_eq!(d.file_count(), 1);
    }

    #[test]
    fn test_files_dict_has_entrypoint() {
        let mut d = FilesDict::new();
        d.insert("run.sh", "#!/bin/sh");
        assert!(d.has_entrypoint());
    }

    #[test]
    fn test_files_dict_no_entrypoint() {
        let mut d = FilesDict::new();
        d.insert("lib.rs", "pub fn x() {}");
        assert!(!d.has_entrypoint());
    }

    #[test]
    fn test_from_llm_output_parses_file() {
        let output = "main.rs\n```rust\nfn main() {}\n```\n";
        let dict = FilesDict::from_llm_output(output);
        assert_eq!(dict.file_count(), 1);
        assert!(dict.get("main.rs").is_some());
    }

    #[test]
    fn test_generate_produces_files() {
        let pipeline = CodeGenPipeline::new();
        let prompt = CodePrompt::new("build a rust cli tool");
        let step = pipeline.generate(&prompt);
        assert!(step.files.file_count() >= 1);
        assert_eq!(step.step, 1);
        assert_eq!(step.mode, GenerationMode::Generate);
    }

    #[test]
    fn test_generate_rust_lang() {
        let pipeline = CodeGenPipeline::new();
        let prompt = CodePrompt::new("rust server");
        let step = pipeline.generate(&prompt);
        assert!(step.files.get("main.rs").is_some());
    }

    #[test]
    fn test_improve_adds_comment() {
        let pipeline = CodeGenPipeline::new();
        let mut files = FilesDict::new();
        files.insert("main.py", "print('hello')");
        let step = pipeline.improve(&mut files, "add error handling");
        assert!(step.files.get("main.py").unwrap().contains("Improved:"));
    }

    #[test]
    fn test_preprompt_build_system_prompt() {
        let p = PrepromptHolder::default();
        let s = p.build_system_prompt(&GenerationMode::Generate);
        assert!(s.contains("step by step"));
    }
}
