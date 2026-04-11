//! Go → Rust pattern-based translator.

use crate::{TranslateResult, confidence_from_untranslated};

/// Translate a Go function body to Rust.
pub fn translate_go_to_rust(body: &str) -> TranslateResult {
    let mut warnings = Vec::new();
    let mut untranslated = 0usize;
    let mut output_lines = Vec::new();

    let lines: Vec<&str> = body.lines().collect();

    for line in &lines {
        let trimmed = line.trim();
        let indent = &line[..line.len() - trimmed.len()];

        // Pass through empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*")
            || trimmed.starts_with("*") || trimmed.starts_with("*/")
        {
            output_lines.push((*line).to_owned());
            continue;
        }

        // import statements
        if trimmed.starts_with("import ") || trimmed == "import (" || trimmed == ")" && output_lines.last().is_some_and(|l| l.trim().starts_with("//")) {
            output_lines.push(format!("{indent}// {trimmed}"));
            continue;
        }

        let mut translated = (*line).to_owned();
        let mut did_translate = false;

        // := → let mut ... =
        if translated.contains(":=") {
            translated = translate_short_assign(&translated);
            did_translate = true;
        }

        // err != nil → err.is_err()
        if translated.contains("err != nil") {
            translated = translated.replace("err != nil", "err.is_err()");
            did_translate = true;
        }
        if translated.contains("== nil") {
            translated = translated.replace("== nil", ".is_none()");
            did_translate = true;
            warnings.push("nil comparison translated to .is_none() — may need .is_null()".into());
        }
        if translated.contains("!= nil") {
            translated = translated.replace("!= nil", ".is_some()");
            did_translate = true;
        }

        // if err != nil { return err } → ? operator
        // Already partially handled above; the full pattern is multi-line

        // fmt.Println → println!
        if translated.contains("fmt.Println(") {
            translated = translate_fmt_println(&translated, "fmt.Println");
            did_translate = true;
        }
        if translated.contains("fmt.Printf(") {
            translated = translate_fmt_println(&translated, "fmt.Printf");
            // Convert Go format verbs
            translated = translated.replace("%d", "{}");
            translated = translated.replace("%s", "{}");
            translated = translated.replace("%v", "{:?}");
            translated = translated.replace("%f", "{}");
            translated = translated.replace("\\n", "");
            did_translate = true;
        }
        if translated.contains("fmt.Sprintf(") {
            translated = translated.replace("fmt.Sprintf(", "format!(");
            translated = translated.replace("%d", "{}");
            translated = translated.replace("%s", "{}");
            translated = translated.replace("%v", "{:?}");
            did_translate = true;
        }

        // len(x) → x.len()
        if let Some(new) = try_translate_go_len(&translated) {
            translated = new;
            did_translate = true;
        }

        // append(x, y) → x.push(y)
        if let Some(new) = try_translate_append(&translated) {
            translated = new;
            did_translate = true;
        }

        // for range patterns
        if let Some(rust_for) = try_translate_go_for(trimmed) {
            translated = format!("{indent}{rust_for}");
            did_translate = true;
        }

        // go func() { } → tokio::spawn(async { })
        if trimmed.starts_with("go func()") || trimmed.starts_with("go func(") {
            let rest = trimmed.strip_prefix("go ").unwrap();
            translated = format!("{indent}tokio::spawn(async {{ {rest}");
            did_translate = true;
            warnings.push("goroutine translated to tokio::spawn — needs async runtime".into());
        }

        // make(chan T) → tokio::sync::mpsc::channel
        if translated.contains("make(chan ") {
            translated = translate_make_chan(&translated);
            did_translate = true;
        }

        // <- ch (channel receive) → ch.recv().await
        if translated.contains("<- ") {
            translated = translated.replace("<- ", ".recv().await");
            did_translate = true;
        }

        // Go type translations
        translated = translate_go_types(&translated);
        if translated != *line {
            did_translate = true;
        }

        // nil → None
        translated = replace_go_word(&translated, "nil", "None");

        if !did_translate && !trimmed.starts_with("{") && !trimmed.starts_with("}")
            && !trimmed.starts_with("return")
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

/// Replace Go word at word boundaries.
fn replace_go_word(s: &str, old: &str, new: &str) -> String {
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

/// Translate `:=` to `let mut`.
fn translate_short_assign(line: &str) -> String {
    if let Some(pos) = line.find(":=") {
        let before = &line[..pos];
        let after = &line[pos + 2..];
        // Get variable name (last word before :=)
        let var = before.trim();
        let indent_end = line.len() - line.trim_start().len();
        let indent = &line[..indent_end];
        format!("{indent}let mut {var} ={after};")
    } else {
        line.to_owned()
    }
}

/// Translate `fmt.Println(args)` → `println!("...", args)`.
fn translate_fmt_println(line: &str, func: &str) -> String {
    if let Some(start) = line.find(func) {
        let call_start = start + func.len();
        if line[call_start..].starts_with('(') {
            let after = call_start + 1;
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
                if args.starts_with('"') {
                    return format!("{before}println!({args}){after_close}");
                }
                return format!("{before}println!(\"{{}}\", {args}){after_close}");
            }
        }
    }
    line.to_owned()
}

/// Try to translate `len(x)` → `x.len()`.
fn try_translate_go_len(line: &str) -> Option<String> {
    let start = line.find("len(")?;
    // Make sure it's not part of another word
    if start > 0 && line.as_bytes()[start - 1].is_ascii_alphanumeric() {
        return None;
    }
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

/// Try to translate `append(slice, elem)` → `slice.push(elem)`.
fn try_translate_append(line: &str) -> Option<String> {
    let start = line.find("append(")?;
    if start > 0 && line.as_bytes()[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    let after = start + 7;
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
    let args = &line[after..end];
    let comma = args.find(',')?;
    let slice_name = args[..comma].trim();
    let elem = args[comma + 1..].trim();
    let before = &line[..start];
    let after_close = &line[end + 1..];
    Some(format!("{before}{slice_name}.push({elem}){after_close}"))
}

/// Try to translate Go for-range loops.
fn try_translate_go_for(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("for ") {
        return None;
    }

    let rest = trimmed.strip_prefix("for ")?.strip_suffix(" {")
        .or_else(|| trimmed.strip_prefix("for ")?.strip_suffix('{'))?
        .trim();

    // for i, v := range x → for (i, v) in x.iter().enumerate()
    if rest.contains(":= range ") {
        let parts: Vec<&str> = rest.splitn(2, ":= range ").collect();
        if parts.len() == 2 {
            let vars = parts[0].trim();
            let collection = parts[1].trim();

            if vars.contains(',') {
                let var_parts: Vec<&str> = vars.split(',').map(|s| s.trim()).collect();
                if var_parts.len() == 2 {
                    if var_parts[0] == "_" {
                        // for _, v := range x → for v in &x
                        return Some(format!("for {} in &{collection} {{", var_parts[1]));
                    }
                    // for i, v := range x → for (i, v) in x.iter().enumerate()
                    return Some(format!(
                        "for ({}, {}) in {collection}.iter().enumerate() {{",
                        var_parts[0], var_parts[1]
                    ));
                }
            } else {
                // for v := range x → for v in &x
                return Some(format!("for {vars} in &{collection} {{"));
            }
        }
    }

    None
}

/// Translate Go type names to Rust.
fn translate_go_types(line: &str) -> String {
    let mut s = line.to_owned();
    s = replace_go_word(&s, "string", "String");
    // Only replace standalone "int" not "interface" etc.
    s = replace_go_type(&s, "int", "i64");
    s = replace_go_type(&s, "int8", "i8");
    s = replace_go_type(&s, "int16", "i16");
    s = replace_go_type(&s, "int32", "i32");
    s = replace_go_type(&s, "int64", "i64");
    s = replace_go_type(&s, "uint", "u64");
    s = replace_go_type(&s, "uint8", "u8");
    s = replace_go_type(&s, "uint16", "u16");
    s = replace_go_type(&s, "uint32", "u32");
    s = replace_go_type(&s, "uint64", "u64");
    s = replace_go_type(&s, "float32", "f32");
    s = replace_go_type(&s, "float64", "f64");
    s = replace_go_type(&s, "bool", "bool");
    s = s.replace("[]byte", "Vec<u8>");
    s
}

/// Replace a Go type name, being careful about word boundaries.
fn replace_go_type(s: &str, old: &str, new: &str) -> String {
    replace_go_word(s, old, new)
}

/// Translate `make(chan T)` → `tokio::sync::mpsc::channel::<T>(32)`.
fn translate_make_chan(line: &str) -> String {
    if let Some(start) = line.find("make(chan ") {
        let after = start + 10; // "make(chan ".len()
        if let Some(end) = line[after..].find(')') {
            let type_name = line[after..after + end].trim();
            let before = &line[..start];
            let after_close = &line[after + end + 1..];
            return format!(
                "{before}tokio::sync::mpsc::channel::<{type_name}>(32){after_close}"
            );
        }
    }
    line.to_owned()
}
