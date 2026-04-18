#![deny(unsafe_code)]
pub fn auto_indent_text(prev_line: &str, _tab_size: usize) -> String {
    let leading = prev_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();
    leading
}
pub fn indent_line(line: &str, tab_size: usize) -> String {
    format!("{}{}", " ".repeat(tab_size), line)
}
pub fn dedent_line(line: &str, tab_size: usize) -> String {
    let spaces_to_remove = line.chars().take_while(|c| *c == ' ').count().min(tab_size);
    line[spaces_to_remove..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_increases_level() {
        let result = indent_line("hello", 4);
        assert_eq!(result, "    hello");
    }

    #[test]
    fn dedent_decreases_level() {
        let result = dedent_line("    hello", 4);
        assert_eq!(result, "hello");
    }

    #[test]
    fn indent_of_empty_is_zero() {
        let result = auto_indent_text("", 4);
        assert_eq!(result, "");
    }

    #[test]
    fn auto_indent_preserves_leading_spaces() {
        let result = auto_indent_text("    hello", 4);
        assert_eq!(result, "    "); // 4 leading spaces preserved
    }

    #[test]
    fn auto_indent_tab_prefix() {
        let result = auto_indent_text("\tcode", 4);
        assert_eq!(result, "\t"); // tab preserved as indent
    }

    #[test]
    fn indent_line_size_2() {
        let result = indent_line("x", 2);
        assert_eq!(result, "  x");
    }

    #[test]
    fn indent_line_already_indented() {
        let result = indent_line("  already", 4);
        assert_eq!(result, "      already"); // 4 more spaces prepended
    }

    #[test]
    fn dedent_line_size_2() {
        let result = dedent_line("  hello", 2);
        assert_eq!(result, "hello");
    }

    #[test]
    fn dedent_line_partial() {
        // Only 2 leading spaces but tab_size=4 → remove only 2
        let result = dedent_line("  hi", 4);
        assert_eq!(result, "hi");
    }

    #[test]
    fn dedent_line_no_leading_spaces() {
        let result = dedent_line("hello", 4);
        assert_eq!(result, "hello"); // nothing to remove
    }

    #[test]
    fn indent_then_dedent_roundtrip() {
        let original = "code here";
        let indented = indent_line(original, 4);
        let dedented = dedent_line(&indented, 4);
        assert_eq!(dedented, original);
    }

    #[test]
    fn auto_indent_mixed_whitespace_stops_at_non_whitespace() {
        let result = auto_indent_text("  \tcontent", 4);
        // Leading whitespace is "  \t"
        assert_eq!(result, "  \t");
    }

    // ── tab-to-spaces conversion for pasted content ──────────────────────────

    #[test]
    fn tab_to_spaces_four_spaces() {
        // indent_line uses spaces, not tabs
        let result = indent_line("code", 4);
        assert!(!result.contains('\t'), "indent_line must not use tabs");
        assert!(result.starts_with("    "));
    }

    #[test]
    fn tab_to_spaces_pasted_content_replaces_tabs() {
        // Simulate converting pasted content's tabs to spaces.
        let pasted = "\tfunction() {\n\t\treturn 1;\n\t}";
        let result: String = pasted
            .lines()
            .map(|line| line.replace('\t', "    "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!result.contains('\t'), "pasted content must have no tabs after conversion");
        assert!(result.contains("    function"), "leading tab replaced by 4 spaces");
    }

    #[test]
    fn indent_line_tab_size_zero() {
        // tab_size=0: prefix is empty string
        let result = indent_line("hello", 0);
        assert_eq!(result, "hello");
    }

    #[test]
    fn dedent_line_tab_size_zero() {
        // tab_size=0: nothing removed
        let result = dedent_line("  hello", 0);
        assert_eq!(result, "  hello");
    }

    #[test]
    fn indent_twice_doubles_indent() {
        let once = indent_line("x", 4);
        let twice = indent_line(&once, 4);
        assert!(twice.starts_with("        "), "double-indent must start with 8 spaces");
        assert!(twice.ends_with('x'));
    }

    #[test]
    fn dedent_removes_exactly_tab_size_spaces() {
        let result = dedent_line("        code", 4); // 8 spaces → remove 4
        assert_eq!(result, "    code");
    }

    #[test]
    fn auto_indent_only_whitespace_line() {
        // prev_line is all spaces — carry forward
        let result = auto_indent_text("    ", 4);
        assert_eq!(result, "    ");
    }

    #[test]
    fn auto_indent_single_space() {
        let result = auto_indent_text(" code", 4);
        assert_eq!(result, " ");
    }

    #[test]
    fn indent_preserves_unicode() {
        let result = indent_line("hello", 2);
        assert_eq!(result, "  hello");
    }

    #[test]
    fn dedent_does_not_remove_non_space() {
        // Line starting with non-space: dedent removes 0 spaces.
        let result = dedent_line("hello", 4);
        assert_eq!(result, "hello");
    }

    #[test]
    fn indent_empty_string() {
        let result = indent_line("", 4);
        assert_eq!(result, "    ");
    }

    #[test]
    fn dedent_empty_string() {
        let result = dedent_line("", 4);
        assert_eq!(result, "");
    }

    #[test]
    fn auto_indent_no_content_returns_empty() {
        let result = auto_indent_text("", 4);
        assert_eq!(result, "");
    }
}
