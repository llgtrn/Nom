//! JavaScript/TypeScript → Rust pattern-based translator.

use crate::{TranslateResult, confidence_from_untranslated};

/// Translate a JavaScript or TypeScript function body to Rust.
pub fn translate_js_to_rust(body: &str) -> TranslateResult {
    let mut warnings = Vec::new();
    let mut untranslated = 0usize;
    let mut output_lines = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        let indent = &line[..line.len() - trimmed.len()];

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

        // import/export statements
        if trimmed.starts_with("import ") {
            output_lines.push(format!("{indent}// {trimmed}"));
            continue;
        }
        if trimmed.starts_with("export default ") {
            let rest = trimmed.strip_prefix("export default ").unwrap();
            output_lines.push(format!("{indent}pub {rest}"));
            continue;
        }
        if trimmed.starts_with("export ") {
            let rest = trimmed.strip_prefix("export ").unwrap();
            output_lines.push(format!("{indent}pub {rest}"));
            continue;
        }

        let mut translated = line.to_owned();
        let mut did_translate = false;

        // const/let/var → let
        if trimmed.starts_with("const ") || trimmed.starts_with("var ") {
            translated = translated.replacen("const ", "let ", 1);
            translated = translated.replacen("var ", "let ", 1);
            did_translate = true;
        } else if trimmed.starts_with("let ") {
            // let is same in both languages, keep as-is
            did_translate = true;
        }

        // === → ==, !== → !=
        if translated.contains("===") {
            translated = translated.replace("===", "==");
            did_translate = true;
        }
        if translated.contains("!==") {
            translated = translated.replace("!==", "!=");
            did_translate = true;
        }

        // null/undefined → None
        translated = replace_js_word(&translated, "null", "None");
        translated = replace_js_word(&translated, "undefined", "None");

        // console.log(x) → println!("{}", x)
        if translated.contains("console.log(") {
            translated = translate_console_log(&translated);
            did_translate = true;
        }

        // .length → .len()
        if translated.contains(".length") {
            translated = translate_dot_length(&translated);
            did_translate = true;
        }

        // .push( → .push( (same in Rust)
        if translated.contains(".push(") {
            did_translate = true;
        }

        // for (const x of y) → for x in y
        if let Some(rust_for) = try_translate_for_of(trimmed) {
            translated = format!("{indent}{rust_for}");
            did_translate = true;
        }

        // for (let i = 0; i < n; i++) → for i in 0..n
        if let Some(rust_for) = try_translate_c_style_for(trimmed) {
            translated = format!("{indent}{rust_for}");
            did_translate = true;
        }

        // async function → async fn
        if translated.contains("async function ") {
            translated = translated.replace("async function ", "async fn ");
            did_translate = true;
        }
        // function → fn
        if translated.contains("function ") {
            translated = translated.replace("function ", "fn ");
            did_translate = true;
        }

        // await x → x.await
        if translated.contains("await ") {
            translated = translate_await(&translated);
            did_translate = true;
        }

        // Arrow functions: (x) => y → |x| y
        if translated.contains("=>") {
            translated = translate_arrow(&translated);
            did_translate = true;
        }

        // TypeScript type annotations
        translated = translate_ts_types(&translated);

        // interface X { → struct X {
        if trimmed.starts_with("interface ") {
            translated = translated.replacen("interface ", "struct ", 1);
            did_translate = true;
        }

        // try { → match (comment)
        if trimmed == "try {" {
            output_lines.push(format!("{indent}// try {{"));
            warnings.push("try/catch needs manual conversion to Result/match".into());
            continue;
        }
        if trimmed.starts_with("} catch") {
            output_lines.push(format!("{indent}// }} catch {{"));
            continue;
        }

        if translated != line.to_owned() {
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

/// Replace a JS word at word boundaries.
fn replace_js_word(s: &str, old: &str, new: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;

    while let Some(pos) = remaining.find(old) {
        let before_ok = pos == 0
            || !remaining.as_bytes()[pos - 1].is_ascii_alphanumeric()
                && remaining.as_bytes()[pos - 1] != b'_';
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

/// Translate `console.log(args)` → `println!("{}", args)`.
fn translate_console_log(line: &str) -> String {
    if let Some(start) = line.find("console.log(") {
        let after = start + 12;
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
                let clean = args.replace('\'', "\"");
                return format!("{before}println!({clean}){after_close}");
            }
            return format!("{before}println!(\"{{}}\", {args}){after_close}");
        }
    }
    line.to_owned()
}

/// Translate `.length` to `.len()` (but not `.length()`).
fn translate_dot_length(line: &str) -> String {
    let mut result = String::new();
    let mut remaining = line;

    while let Some(pos) = remaining.find(".length") {
        let after = pos + 7; // ".length".len()
        // Don't translate if followed by '(' — it's already a method call
        let is_method = after < remaining.len() && remaining.as_bytes()[after] == b'(';
        if is_method {
            result.push_str(&remaining[..after]);
            remaining = &remaining[after..];
        } else {
            // Check it's not ".lengthy" or similar
            let after_ok =
                after >= remaining.len() || !remaining.as_bytes()[after].is_ascii_alphanumeric();
            if after_ok {
                result.push_str(&remaining[..pos]);
                result.push_str(".len()");
                remaining = &remaining[after..];
            } else {
                result.push_str(&remaining[..after]);
                remaining = &remaining[after..];
            }
        }
    }
    result.push_str(remaining);
    result
}

/// Try to translate `for (const x of y)` → `for x in y {`.
fn try_translate_for_of(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("for") {
        return None;
    }
    let rest = trimmed.strip_prefix("for")?.trim();
    let inner = rest
        .strip_prefix('(')?
        .strip_suffix(") {")
        .or_else(|| rest.strip_prefix('(')?.strip_suffix(')'))?;

    if !inner.contains(" of ") {
        return None;
    }

    let parts: Vec<&str> = inner.splitn(2, " of ").collect();
    if parts.len() != 2 {
        return None;
    }

    let var = parts[0]
        .trim()
        .strip_prefix("const ")
        .or_else(|| parts[0].trim().strip_prefix("let "))
        .or_else(|| parts[0].trim().strip_prefix("var "))
        .unwrap_or(parts[0].trim());
    let iter = parts[1].trim();

    Some(format!("for {var} in {iter} {{"))
}

/// Try to translate C-style for loop from JS.
fn try_translate_c_style_for(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("for") {
        return None;
    }
    let rest = trimmed.strip_prefix("for")?.trim();
    let inner = rest
        .strip_prefix('(')?
        .strip_suffix(") {")
        .or_else(|| rest.strip_prefix('(')?.strip_suffix(')'))?;

    if inner.contains(" of ") || inner.contains(" in ") {
        return None;
    }

    let parts: Vec<&str> = inner.split(';').collect();
    if parts.len() != 3 {
        return None;
    }

    let init = parts[0].trim();
    let cond = parts[1].trim();
    let incr = parts[2].trim();

    // Parse init: "let i = 0" or "var i = 0"
    let init_clean = init
        .strip_prefix("let ")
        .or_else(|| init.strip_prefix("var "))
        .or_else(|| init.strip_prefix("const "))
        .unwrap_or(init);
    let init_parts: Vec<&str> = init_clean.split('=').collect();
    if init_parts.len() != 2 {
        return None;
    }
    let var_name = init_parts[0].trim();
    let start = init_parts[1].trim();

    // Parse condition
    let (end, inclusive) = if cond.contains("<=") {
        let parts: Vec<&str> = cond.split("<=").collect();
        (parts.get(1)?.trim(), true)
    } else if cond.contains('<') {
        let parts: Vec<&str> = cond.split('<').collect();
        (parts.get(1)?.trim(), false)
    } else {
        return None;
    };

    // Verify simple increment
    let valid = incr == format!("{var_name}++")
        || incr == format!("++{var_name}")
        || incr == format!("{var_name} += 1");
    if !valid {
        return None;
    }

    let range = if start == "0" {
        if inclusive {
            format!("0..={end}")
        } else {
            format!("0..{end}")
        }
    } else if inclusive {
        format!("{start}..={end}")
    } else {
        format!("{start}..{end}")
    };

    Some(format!("for {var_name} in {range} {{"))
}

/// Translate `await expr` → `expr.await`.
fn translate_await(line: &str) -> String {
    // Simple case: replace "await X" with "X.await"
    let mut result = line.to_owned();
    while let Some(pos) = result.find("await ") {
        let before_ok = pos == 0 || !result.as_bytes()[pos - 1].is_ascii_alphanumeric();
        if !before_ok {
            break;
        }
        let after = pos + 6;
        // Find the end of the expression (semicolon, comma, closing paren)
        let mut end = result.len();
        for (i, ch) in result[after..].char_indices() {
            if ch == ';' || ch == ',' || ch == ')' || ch == '}' {
                end = after + i;
                break;
            }
        }
        let expr = result[after..end].trim();
        result = format!("{}{expr}.await{}", &result[..pos], &result[end..]);
    }
    result
}

/// Translate arrow functions: `(x) => y` → `|x| y`.
fn translate_arrow(line: &str) -> String {
    let mut result = line.to_owned();
    // Simple replacement: `=> ` → ` ` with params wrapped in ||
    // This is very mechanical and won't handle all cases
    if let Some(arrow_pos) = result.find(" => ") {
        // Look backwards for the params
        let before = &result[..arrow_pos];
        if let Some(paren_start) = before.rfind('(') {
            let params = &before[paren_start + 1..arrow_pos].trim_end_matches(')');
            let prefix = &before[..paren_start];
            let after = &result[arrow_pos + 4..];
            result = format!("{prefix}|{params}| {after}");
        }
    }
    result
}

/// Translate TypeScript type annotations.
fn translate_ts_types(line: &str) -> String {
    let mut s = line.to_owned();
    s = s.replace(": string", ": String");
    s = s.replace(": number", ": f64");
    s = s.replace(": boolean", ": bool");
    s = s.replace(": void", ": ()");
    s = s.replace(": any", ": _");
    s
}
