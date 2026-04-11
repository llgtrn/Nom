//! C → Rust pattern-based translator.

use crate::{TranslateResult, confidence_from_untranslated};

/// Translate a C function body to Rust using mechanical pattern replacements.
pub fn translate_c_to_rust(body: &str) -> TranslateResult {
    let mut warnings = Vec::new();
    let mut untranslated = 0usize;
    let mut output_lines = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // Skip preprocessor directives
        if trimmed.starts_with("#include") || trimmed.starts_with("#define")
            || trimmed.starts_with("#ifdef") || trimmed.starts_with("#ifndef")
            || trimmed.starts_with("#endif") || trimmed.starts_with("#pragma")
        {
            output_lines.push(format!("// {trimmed}"));
            continue;
        }

        // Skip empty lines and pass comments through
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*")
            || trimmed.starts_with("*") || trimmed.starts_with("*/")
        {
            output_lines.push(line.to_owned());
            continue;
        }

        let mut translated = line.to_owned();
        let mut did_translate = false;

        // C-style for loop → Rust for..in
        if let Some(rust_for) = try_translate_c_for_loop(trimmed) {
            let indent = &line[..line.len() - trimmed.len()];
            translated = format!("{indent}{rust_for}");
            did_translate = true;
        } else {
            // Type replacements
            translated = translate_c_types(&translated);

            // NULL → std::ptr::null()
            if translated.contains("NULL") {
                translated = translated.replace("NULL", "std::ptr::null()");
                did_translate = true;
            }

            // malloc(n) → Vec::with_capacity(n)
            if translated.contains("malloc(") {
                translated = replace_func_call(&translated, "malloc", "Vec::with_capacity");
                did_translate = true;
            }

            // free(p) → drop(p)
            if translated.contains("free(") {
                translated = replace_func_call(&translated, "free", "drop");
                did_translate = true;
            }

            // sizeof(T) → std::mem::size_of::<T>()
            if let Some(new) = try_replace_sizeof(&translated) {
                translated = new;
                did_translate = true;
            }

            // printf("...") → println!("...")
            if translated.contains("printf(") {
                translated = replace_func_call(&translated, "printf", "println!");
                // Remove format specifiers in a basic way
                translated = translated.replace("%d", "{}");
                translated = translated.replace("%s", "{}");
                translated = translated.replace("%f", "{}");
                translated = translated.replace("%u", "{}");
                translated = translated.replace("%ld", "{}");
                translated = translated.replace("%lu", "{}");
                translated = translated.replace("%x", "{:x}");
                translated = translated.replace("%p", "{:p}");
                translated = translated.replace("\\n", "");
                did_translate = true;
            }

            // -> (pointer member access) → .
            if translated.contains("->") {
                translated = translated.replace("->", ".");
                did_translate = true;
            }

            // typedef → type
            if trimmed.starts_with("typedef ") {
                translated = translated.replacen("typedef ", "type ", 1);
                did_translate = true;
                warnings.push("typedef translated to type alias — may need adjustment".into());
            }

            // Check if types changed
            if translated != line.to_owned() {
                did_translate = true;
            }
        }

        if !did_translate && !trimmed.starts_with("{") && !trimmed.starts_with("}")
            && !trimmed.starts_with("return") && trimmed != ";"
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

/// Translate C type names to Rust equivalents within a line.
pub(crate) fn translate_c_types(line: &str) -> String {
    let mut s = line.to_owned();

    // Order matters: longer patterns first
    s = replace_word(&s, "unsigned long long", "u64");
    s = replace_word(&s, "unsigned long", "u64");
    s = replace_word(&s, "unsigned int", "u32");
    s = replace_word(&s, "unsigned short", "u16");
    s = replace_word(&s, "unsigned char", "u8");
    s = replace_word(&s, "long long", "i64");
    s = replace_word(&s, "long", "i64");
    s = replace_word(&s, "short", "i16");
    s = replace_word(&s, "size_t", "usize");

    // Single-word types (only replace when they look like type declarations)
    // "int x" → "i32 x", but not "print" containing "int"
    s = replace_c_type_word(&s, "int", "i32");
    s = replace_c_type_word(&s, "float", "f32");
    s = replace_c_type_word(&s, "double", "f64");
    s = replace_c_type_word(&s, "void", "()");

    // char* → *const u8
    s = s.replace("char*", "*const u8");
    s = s.replace("char *", "*const u8 ");

    s
}

/// Replace a C type word only at word boundaries in type position.
fn replace_c_type_word(line: &str, c_type: &str, rust_type: &str) -> String {
    let mut result = String::new();
    let mut remaining = line;

    while let Some(pos) = remaining.find(c_type) {
        // Check word boundaries
        let before_ok = pos == 0 || !remaining.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let after_pos = pos + c_type.len();
        let after_ok = after_pos >= remaining.len()
            || (!remaining.as_bytes()[after_pos].is_ascii_alphanumeric()
                && remaining.as_bytes()[after_pos] != b'_');

        if before_ok && after_ok {
            result.push_str(&remaining[..pos]);
            result.push_str(rust_type);
            remaining = &remaining[after_pos..];
        } else {
            result.push_str(&remaining[..after_pos]);
            remaining = &remaining[after_pos..];
        }
    }
    result.push_str(remaining);
    result
}

/// Replace a multi-word pattern at word boundaries.
fn replace_word(line: &str, pattern: &str, replacement: &str) -> String {
    if !line.contains(pattern) {
        return line.to_owned();
    }

    let mut result = String::new();
    let mut remaining = line;

    while let Some(pos) = remaining.find(pattern) {
        let before_ok = pos == 0 || !remaining.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let after_pos = pos + pattern.len();
        let after_ok = after_pos >= remaining.len()
            || !remaining.as_bytes()[after_pos].is_ascii_alphanumeric();

        if before_ok && after_ok {
            result.push_str(&remaining[..pos]);
            result.push_str(replacement);
            remaining = &remaining[after_pos..];
        } else {
            result.push_str(&remaining[..after_pos]);
            remaining = &remaining[after_pos..];
        }
    }
    result.push_str(remaining);
    result
}

/// Try to translate `for (int i = 0; i < n; i++)` → `for i in 0..n`.
fn try_translate_c_for_loop(trimmed: &str) -> Option<String> {
    // Match: for (int i = START; i < END; i++) {
    if !trimmed.starts_with("for") {
        return None;
    }
    let rest = trimmed.strip_prefix("for")?.trim();
    let inner = rest.strip_prefix('(')?.strip_suffix(") {")
        .or_else(|| rest.strip_prefix('(')?.strip_suffix(')'))?;

    let parts: Vec<&str> = inner.split(';').collect();
    if parts.len() != 3 {
        return None;
    }

    let init = parts[0].trim();
    let cond = parts[1].trim();
    let incr = parts[2].trim();

    // Parse init: "int i = 0" or "i = 0"
    let init_parts: Vec<&str> = init.split('=').collect();
    if init_parts.len() != 2 {
        return None;
    }
    let var_part = init_parts[0].trim();
    let start = init_parts[1].trim();

    // Extract variable name (last word of var_part)
    let var_name = var_part.split_whitespace().last()?;

    // Parse condition: "i < n"
    let cond_parts: Vec<&str> = if cond.contains("<=") {
        cond.split("<=").collect()
    } else if cond.contains('<') {
        cond.split('<').collect()
    } else {
        return None;
    };
    if cond_parts.len() != 2 {
        return None;
    }
    let end = cond_parts[1].trim();
    let inclusive = cond.contains("<=");

    // Verify increment is simple i++ or i += 1
    let valid_incr = incr == format!("{var_name}++")
        || incr == format!("++{var_name}")
        || incr == format!("{var_name} += 1");
    if !valid_incr {
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

/// Replace a function call: `old_func(args)` → `new_func(args)`.
pub(crate) fn replace_func_call(line: &str, old_func: &str, new_func: &str) -> String {
    line.replace(&format!("{old_func}("), &format!("{new_func}("))
}

/// Try to replace `sizeof(T)` with `std::mem::size_of::<T>()`.
fn try_replace_sizeof(line: &str) -> Option<String> {
    if !line.contains("sizeof(") {
        return None;
    }
    let mut result = line.to_owned();
    while let Some(start) = result.find("sizeof(") {
        let after = start + 7; // len("sizeof(")
        if let Some(end) = result[after..].find(')') {
            let type_name = result[after..after + end].trim().to_owned();
            let replacement = format!("std::mem::size_of::<{type_name}>()");
            result = format!("{}{}{}", &result[..start], replacement, &result[after + end + 1..]);
        } else {
            break;
        }
    }
    Some(result)
}
