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
}
