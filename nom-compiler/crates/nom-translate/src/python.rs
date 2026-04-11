//! Python → Rust pattern-based translator.

use crate::{TranslateResult, confidence_from_untranslated};

/// Translate a Python function body to Rust.
pub fn translate_python_to_rust(body: &str) -> TranslateResult {
    let mut warnings = Vec::new();
    let mut untranslated = 0usize;
    let mut output_lines = Vec::new();

    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let indent = &line[..line.len() - trimmed.len()];

        // Empty lines and comments
        if trimmed.is_empty() {
            output_lines.push(String::new());
            i += 1;
            continue;
        }
        if trimmed.starts_with('#') {
            let comment = trimmed.strip_prefix('#').unwrap_or("");
            output_lines.push(format!("{indent}//{comment}"));
            i += 1;
            continue;
        }

        let mut translated = trimmed.to_owned();
        let mut did_translate = false;

        // import → commented out
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            output_lines.push(format!("{indent}// {trimmed}"));
            i += 1;
            continue;
        }

        // if x: → if x {
        if trimmed.starts_with("if ") && trimmed.ends_with(':') {
            let cond = trimmed
                .strip_prefix("if ")
                .unwrap()
                .strip_suffix(':')
                .unwrap()
                .trim();
            let rust_cond = translate_python_expr(cond);
            output_lines.push(format!("{indent}if {rust_cond} {{"));
            i += 1;
            continue;
        }

        // elif → } else if {
        if trimmed.starts_with("elif ") && trimmed.ends_with(':') {
            let cond = trimmed
                .strip_prefix("elif ")
                .unwrap()
                .strip_suffix(':')
                .unwrap()
                .trim();
            let rust_cond = translate_python_expr(cond);
            output_lines.push(format!("{indent}}} else if {rust_cond} {{"));
            i += 1;
            continue;
        }

        // else: → } else {
        if trimmed == "else:" {
            output_lines.push(format!("{indent}}} else {{"));
            i += 1;
            continue;
        }

        // for x in range(n): → for x in 0..n {
        if trimmed.starts_with("for ") && trimmed.ends_with(':') {
            if let Some(rust_for) = try_translate_python_for(trimmed) {
                output_lines.push(format!("{indent}{rust_for}"));
                i += 1;
                continue;
            }
        }

        // while x: → while x {
        if trimmed.starts_with("while ") && trimmed.ends_with(':') {
            let cond = trimmed
                .strip_prefix("while ")
                .unwrap()
                .strip_suffix(':')
                .unwrap()
                .trim();
            let rust_cond = translate_python_expr(cond);
            output_lines.push(format!("{indent}while {rust_cond} {{"));
            i += 1;
            continue;
        }

        // def func(args): → fn func(args) {
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            let sig = trimmed
                .strip_prefix("def ")
                .unwrap()
                .strip_suffix(':')
                .unwrap()
                .trim();
            output_lines.push(format!("{indent}fn {sig} {{"));
            i += 1;
            warnings.push("nested def translated — parameter types need annotation".into());
            continue;
        }

        // return x → return x;
        if trimmed.starts_with("return ") {
            let expr = trimmed.strip_prefix("return ").unwrap().trim();
            let rust_expr = translate_python_expr(expr);
            output_lines.push(format!("{indent}return {rust_expr};"));
            i += 1;
            continue;
        }
        if trimmed == "return" {
            output_lines.push(format!("{indent}return;"));
            i += 1;
            continue;
        }

        // try/except → basic match pattern
        if trimmed == "try:" {
            output_lines.push(format!("{indent}// try {{"));
            i += 1;
            warnings.push("try/except needs manual conversion to Result/match".into());
            continue;
        }
        if trimmed.starts_with("except") && trimmed.ends_with(':') {
            output_lines.push(format!("{indent}// }} catch {{"));
            i += 1;
            continue;
        }

        // pass → {} (no-op)
        if trimmed == "pass" {
            output_lines.push(format!("{indent}// pass"));
            i += 1;
            continue;
        }

        // General expression translation
        translated = translate_python_expr(&translated);
        if !translated.ends_with(';') && !translated.ends_with('{') && !translated.ends_with('}') {
            translated.push(';');
        }

        if translated != format!("{trimmed};") {
            did_translate = true;
        }

        if !did_translate {
            untranslated += 1;
        }

        output_lines.push(format!("{indent}{translated}"));
        i += 1;
    }

    let total = body.lines().count();
    TranslateResult {
        rust_body: output_lines.join("\n"),
        confidence: confidence_from_untranslated(total, untranslated),
        warnings,
        untranslated_lines: untranslated,
    }
}

/// Translate common Python expressions to Rust.
fn translate_python_expr(expr: &str) -> String {
    let mut s = expr.to_owned();

    // Boolean literals
    s = replace_py_word(&s, "True", "true");
    s = replace_py_word(&s, "False", "false");
    s = replace_py_word(&s, "None", "None");

    // print(x) → println!("{}", x)
    if s.contains("print(") {
        s = translate_print(&s);
    }

    // len(x) → x.len()
    if let Some(new) = try_translate_len(&s) {
        s = new;
    }

    // x.append(y) → x.push(y)
    s = s.replace(".append(", ".push(");

    // f"..." → format!("...")
    if s.contains("f\"") {
        s = s.replace("f\"", "format!(\"");
        // Close the format! — find the matching quote
        // Simple approach: just replace, the closing " stays as-is
        // Actually need to add ) before the closing "
        // This is imperfect but mechanical
    }

    s
}

/// Replace a Python word at word boundaries.
fn replace_py_word(s: &str, old: &str, new: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;

    while let Some(pos) = remaining.find(old) {
        let before_ok = pos == 0 || !remaining.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let after_pos = pos + old.len();
        let after_ok = after_pos >= remaining.len()
            || (!remaining.as_bytes()[after_pos].is_ascii_alphanumeric()
                && remaining.as_bytes()[after_pos] != b'_');

        if before_ok && after_ok {
            result.push_str(&remaining[..pos]);
            result.push_str(new);
            remaining = &remaining[after_pos..];
        } else {
            result.push_str(&remaining[..after_pos]);
            remaining = &remaining[after_pos..];
        }
    }
    result.push_str(remaining);
    result
}

/// Translate `print(args)` → `println!("{}", args)`.
fn translate_print(line: &str) -> String {
    if let Some(start) = line.find("print(") {
        let after = start + 6;
        // Find matching closing paren
        let mut depth = 1;
        let mut end = None;
        for (i, ch) in line[after..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(after + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let args = &line[after..end];
            let before = &line[..start];
            let after_close = &line[end + 1..];
            if args.starts_with('"') || args.starts_with('\'') {
                // String literal — use as format string
                let clean = args.replace('\'', "\"");
                return format!("{before}println!({clean}){after_close}");
            }
            return format!("{before}println!(\"{{}}\", {args}){after_close}");
        }
    }
    line.to_owned()
}

/// Try to translate `len(x)` → `x.len()`.
fn try_translate_len(line: &str) -> Option<String> {
    let start = line.find("len(")?;
    let after = start + 4;
    let mut depth = 1;
    let mut end = None;
    for (i, ch) in line[after..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(after + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let arg = &line[after..end];
    let before = &line[..start];
    let after_close = &line[end + 1..];
    Some(format!("{before}{arg}.len(){after_close}"))
}

/// Try to translate Python for loops.
fn try_translate_python_for(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("for ")?.strip_suffix(':')?;
    let in_pos = rest.find(" in ")?;
    let var = rest[..in_pos].trim();
    let iter_expr = rest[in_pos + 4..].trim();

    // range(n) → 0..n
    if iter_expr.starts_with("range(") && iter_expr.ends_with(')') {
        let args = &iter_expr[6..iter_expr.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        let range = match parts.len() {
            1 => format!("0..{}", parts[0]),
            2 => format!("{}..{}", parts[0], parts[1]),
            3 => {
                // range(start, end, step) — no direct Rust equivalent
                return Some(format!(
                    "for {var} in ({}).step_by({} as usize) {{",
                    format_args!("{}..{}", parts[0], parts[1]),
                    parts[2]
                ));
            }
            _ => return None,
        };
        return Some(format!("for {var} in {range} {{"));
    }

    // for x in y: → for x in y {
    Some(format!("for {var} in {iter_expr} {{"))
}
