//! C++ → Rust pattern-based translator.
//!
//! Applies C translation rules first, then applies C++-specific patterns.

use crate::c::{replace_func_call, translate_c_types};
use crate::{TranslateResult, confidence_from_untranslated};

/// Translate a C++ function body to Rust.
pub fn translate_cpp_to_rust(body: &str) -> TranslateResult {
    let mut warnings = Vec::new();
    let mut untranslated = 0usize;
    let mut output_lines = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // Skip preprocessor directives
        if trimmed.starts_with("#include")
            || trimmed.starts_with("#define")
            || trimmed.starts_with("#pragma")
        {
            output_lines.push(format!("// {trimmed}"));
            continue;
        }

        // Pass through empty lines and comments
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with("*")
            || trimmed.starts_with("*/")
        {
            output_lines.push(line.to_owned());
            continue;
        }

        let mut translated = line.to_owned();
        let mut did_translate = false;

        // C++ STL types → Rust equivalents (before C type translation)
        if translated.contains("std::string") {
            translated = translated.replace("std::string", "String");
            did_translate = true;
        }
        if translated.contains("std::vector<") {
            translated = translate_std_template(&translated, "std::vector", "Vec");
            did_translate = true;
        }
        if translated.contains("std::map<") {
            translated = translate_std_template(&translated, "std::map", "HashMap");
            did_translate = true;
        }
        if translated.contains("std::unordered_map<") {
            translated = translate_std_template(&translated, "std::unordered_map", "HashMap");
            did_translate = true;
        }
        if translated.contains("std::unique_ptr<") {
            translated = translate_std_template(&translated, "std::unique_ptr", "Box");
            did_translate = true;
        }
        if translated.contains("std::shared_ptr<") {
            translated = translate_std_template(&translated, "std::shared_ptr", "Arc");
            did_translate = true;
        }

        // std::cout << x → println!("{}", x)
        if translated.contains("std::cout") {
            translated = translate_cout(&translated);
            did_translate = true;
        }

        // this-> → self.
        if translated.contains("this->") {
            translated = translated.replace("this->", "self.");
            did_translate = true;
        }

        // new T(...) → Box::new(T::new(...))
        if translated.contains("new ") {
            if let Some(new_line) = try_translate_new(&translated) {
                translated = new_line;
                did_translate = true;
            }
        }

        // delete x → drop(x)
        if trimmed.starts_with("delete ") {
            let var = trimmed
                .strip_prefix("delete ")
                .unwrap()
                .trim_end_matches(';')
                .trim();
            let indent = &line[..line.len() - trimmed.len()];
            translated = format!("{indent}drop({var});");
            did_translate = true;
        }

        // namespace X { → mod X {
        if trimmed.starts_with("namespace ") {
            translated = translated.replacen("namespace ", "mod ", 1);
            did_translate = true;
        }

        // throw X → return Err(X)
        if trimmed.starts_with("throw ") {
            let expr = trimmed
                .strip_prefix("throw ")
                .unwrap()
                .trim_end_matches(';')
                .trim();
            let indent = &line[..line.len() - trimmed.len()];
            translated = format!("{indent}return Err({expr});");
            did_translate = true;
            warnings.push("throw converted to return Err — may need Result return type".into());
        }

        // -> (pointer member) → .
        if translated.contains("->") {
            translated = translated.replace("->", ".");
            did_translate = true;
        }

        // Apply C type translations for remaining patterns
        let after_c = translate_c_types(&translated);
        if after_c != translated {
            translated = after_c;
            did_translate = true;
        }

        // printf → println!
        if translated.contains("printf(") {
            translated = replace_func_call(&translated, "printf", "println!");
            translated = translated.replace("%d", "{}");
            translated = translated.replace("%s", "{}");
            translated = translated.replace("%f", "{}");
            translated = translated.replace("\\n", "");
            did_translate = true;
        }

        if !did_translate
            && !trimmed.starts_with("{")
            && !trimmed.starts_with("}")
            && !trimmed.starts_with("return")
            && trimmed != ";"
        {
            untranslated += 1;
        }

        output_lines.push(translated);
    }

    let total = body.lines().count();
    TranslateResult {
        rust_body: output_lines.join("\n"),
        confidence: confidence_from_untranslated(total, untranslated),
        warnings,
        untranslated_lines: untranslated,
    }
}

/// Translate `std::vector<T>` → `Vec<T>` style templates.
fn translate_std_template(line: &str, cpp_name: &str, rust_name: &str) -> String {
    line.replace(cpp_name, rust_name)
}

/// Translate `std::cout << a << b << std::endl;` → `println!("{} {}", a, b);`
fn translate_cout(line: &str) -> String {
    let trimmed = line.trim();
    let indent = &line[..line.len() - trimmed.len()];

    // Remove std::cout and std::endl, split by <<
    let content = trimmed
        .replace("std::cout", "")
        .replace("std::endl", "")
        .replace(';', "");

    let parts: Vec<&str> = content
        .split("<<")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return format!("{indent}println!();");
    }

    let fmts: Vec<&str> = parts.iter().map(|_| "{}").collect();
    let fmt_str = fmts.join(" ");
    let args = parts.join(", ");
    format!("{indent}println!(\"{fmt_str}\", {args});")
}

/// Try to translate `new T(args)` → `Box::new(T::new(args))`.
fn try_translate_new(line: &str) -> Option<String> {
    let idx = line.find("new ")?;
    let after = &line[idx + 4..];

    // Find the type name and opening paren
    let paren = after.find('(')?;
    let type_name = after[..paren].trim();
    if type_name.is_empty() {
        return None;
    }

    let rest_from_paren = &after[paren..];
    // Find matching closing paren
    let mut depth = 0;
    let mut end = None;
    for (i, ch) in rest_from_paren.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let args = &rest_from_paren[1..end]; // inside parens
    let after_close = &rest_from_paren[end + 1..];
    let before = &line[..idx];

    Some(format!(
        "{before}Box::new({type_name}::new({args})){after_close}"
    ))
}
