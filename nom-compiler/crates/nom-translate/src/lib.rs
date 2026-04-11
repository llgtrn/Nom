//! Pattern-based translation of function bodies from various languages to Rust.
//!
//! The translator applies mechanical regex/string replacements to convert common
//! patterns from C, C++, Python, JavaScript, TypeScript, and Go into Rust equivalents.
//! Translations are approximate — the `confidence` field indicates how much of the
//! body was successfully translated.

mod c;
mod cpp;
mod go;
mod js;
mod python;

/// Result of translating a function body to Rust.
#[derive(Debug, Clone)]
pub struct TranslateResult {
    /// The translated Rust body.
    pub rust_body: String,
    /// Confidence score from 0.0 (untranslated) to 1.0 (fully translated).
    pub confidence: f64,
    /// Warnings about things that may need manual review.
    pub warnings: Vec<String>,
    /// Number of lines left as comments (untranslated).
    pub untranslated_lines: usize,
}

/// Translate a function body from any language to Rust.
///
/// The `from_language` parameter should match the language field in nomtu entries:
/// `"rust"`, `"c"`, `"cpp"`, `"python"`, `"javascript"`, `"typescript"`, `"go"`.
pub fn translate(body: &str, from_language: &str) -> TranslateResult {
    match from_language {
        "rust" => TranslateResult {
            rust_body: body.to_owned(),
            confidence: 1.0,
            warnings: vec![],
            untranslated_lines: 0,
        },
        "c" => c::translate_c_to_rust(body),
        "cpp" | "c++" => cpp::translate_cpp_to_rust(body),
        "python" => python::translate_python_to_rust(body),
        "javascript" | "typescript" => js::translate_js_to_rust(body),
        "go" => go::translate_go_to_rust(body),
        _ => translate_unknown(body, from_language),
    }
}

/// Wrap an untranslatable body as comments with a `todo!()` placeholder.
fn translate_unknown(body: &str, lang: &str) -> TranslateResult {
    let mut rust = String::new();
    rust.push_str(&format!("// TODO: translate from {lang}\n"));
    for line in body.lines() {
        rust.push_str(&format!("// {line}\n"));
    }
    rust.push_str("todo!(\"translate body\")\n");
    TranslateResult {
        rust_body: rust,
        confidence: 0.0,
        warnings: vec![format!("entire body untranslated from {lang}")],
        untranslated_lines: body.lines().count(),
    }
}

/// Helper: compute confidence from the number of lines that were left untranslated.
pub(crate) fn confidence_from_untranslated(total_lines: usize, untranslated: usize) -> f64 {
    if total_lines == 0 {
        return 1.0;
    }
    let translated = total_lines.saturating_sub(untranslated);
    translated as f64 / total_lines as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_passthrough() {
        let body = "let x = 42;\nprintln!(\"{}\", x);";
        let result = translate(body, "rust");
        assert_eq!(result.rust_body, body);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
        assert!(result.warnings.is_empty());
        assert_eq!(result.untranslated_lines, 0);
    }

    #[test]
    fn test_c_function() {
        let body = r#"int x = 10;
unsigned int y = 20;
char* name = NULL;
printf("hello %d\n", x);
for (int i = 0; i < x; i++) {
    y += i;
}
free(name);
return y;"#;
        let result = translate(body, "c");
        assert!(result.confidence > 0.5, "confidence={}", result.confidence);
        assert!(result.rust_body.contains("i32"));
        assert!(result.rust_body.contains("u32"));
        assert!(result.rust_body.contains("println!"));
        assert!(result.rust_body.contains("for i in 0..x"));
        assert!(result.rust_body.contains("drop(name)"));
    }

    #[test]
    fn test_python_function() {
        let body = r#"x = True
if x:
    print("hello")
elif x == False:
    print("bye")
for i in range(10):
    items.append(i)
return None"#;
        let result = translate(body, "python");
        assert!(result.confidence > 0.5, "confidence={}", result.confidence);
        assert!(result.rust_body.contains("true"));
        assert!(result.rust_body.contains("if x {"));
        assert!(result.rust_body.contains("println!"));
        assert!(result.rust_body.contains("0..10"));
        assert!(result.rust_body.contains(".push(i)"));
    }

    #[test]
    fn test_javascript_function() {
        let body = r#"const x = 10;
let y = null;
console.log("hello");
if (x === y) {
    return undefined;
}
const arr = [];
arr.push(x);
for (const item of arr) {
    console.log(item.length);
}"#;
        let result = translate(body, "javascript");
        assert!(result.confidence > 0.5, "confidence={}", result.confidence);
        assert!(result.rust_body.contains("let x = 10;"));
        assert!(result.rust_body.contains("None"));
        assert!(result.rust_body.contains("println!"));
        assert!(result.rust_body.contains("=="));
        assert!(!result.rust_body.contains("==="));
        assert!(result.rust_body.contains(".len()"));
    }

    #[test]
    fn test_go_function() {
        let body = r#"x := 10
err := doSomething()
if err != nil {
    return err
}
fmt.Println("done")
for i, v := range items {
    fmt.Println(i, v)
}"#;
        let result = translate(body, "go");
        assert!(result.confidence > 0.5, "confidence={}", result.confidence);
        assert!(result.rust_body.contains("let mut x = 10;"));
        assert!(result.rust_body.contains("println!"));
        assert!(result.rust_body.contains("enumerate"));
    }

    #[test]
    fn test_unknown_language() {
        let body = "some code here\nanother line";
        let result = translate(body, "haskell");
        assert!((result.confidence - 0.0).abs() < f64::EPSILON);
        assert_eq!(result.untranslated_lines, 2);
        assert!(result.rust_body.contains("// TODO: translate from haskell"));
        assert!(result.rust_body.contains("todo!(\"translate body\")"));
    }
}
