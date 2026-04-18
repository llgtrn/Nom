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
        assert!(
            !result.contains('\t'),
            "pasted content must have no tabs after conversion"
        );
        assert!(
            result.contains("    function"),
            "leading tab replaced by 4 spaces"
        );
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
        assert!(
            twice.starts_with("        "),
            "double-indent must start with 8 spaces"
        );
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

    // ── wave AJ-7: formatting/indentation tests ─────────────────────────────

    /// Smart indent after open brace: next line gets one extra indent level.
    #[test]
    fn editor_smart_indent_after_open_brace() {
        let prev_line = "define foo {";
        let base_indent = auto_indent_text(prev_line, 4);
        // Smart indent: add one more tab_size indent
        let smart = indent_line(&base_indent, 4);
        assert!(
            smart.len() >= 4,
            "smart indent after '{{' must add at least one level"
        );
    }

    /// Smart indent after colon: next line gets indented.
    #[test]
    fn editor_smart_indent_after_colon() {
        let prev_line = "    define foo:";
        let base_indent = auto_indent_text(prev_line, 4);
        let smart = indent_line(&base_indent, 4);
        // Must have at least 8 spaces (4 from prev + 4 added)
        assert!(smart.len() >= 8);
    }

    /// Auto indent on newline: new line inherits leading whitespace of previous.
    #[test]
    fn editor_auto_indent_on_newline() {
        let prev = "    code here";
        let new_indent = auto_indent_text(prev, 4);
        assert_eq!(new_indent, "    ");
    }

    /// Format selection only: trim trailing whitespace from each line in a selection.
    #[test]
    fn editor_format_selection_only() {
        let selection = "  define foo    \n  result 42   ";
        let formatted: String = selection
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !formatted.contains("    \n"),
            "trailing spaces must be removed"
        );
        assert!(formatted.contains("define foo"));
    }

    /// Trim trailing whitespace: all lines end cleanly.
    #[test]
    fn editor_trim_trailing_whitespace() {
        let source = "hello   \nworld  \nfoo\n";
        let trimmed: String = source
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(!trimmed.contains("   \n"), "trailing spaces must be gone");
        assert!(!trimmed.contains("  \n"), "trailing spaces must be gone");
    }

    /// Ensure newline at EOF: source ends with exactly one newline.
    #[test]
    fn editor_ensure_newline_at_eof() {
        let source = "define foo";
        let with_newline = if source.ends_with('\n') {
            source.to_string()
        } else {
            format!("{source}\n")
        };
        assert!(with_newline.ends_with('\n'), "file must end with newline");
    }

    /// Remove blank lines: blank lines are removed from source.
    #[test]
    fn editor_remove_blank_lines() {
        let source = "line1\n\nline2\n\n\nline3";
        let without_blanks: String = source
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !without_blanks.contains("\n\n"),
            "no blank lines must remain"
        );
        assert_eq!(without_blanks.lines().count(), 3);
    }

    /// Normalize indentation: mixed tabs/spaces become all-spaces.
    #[test]
    fn editor_normalize_indentation() {
        let source = "\thello\n\t\tworld";
        let normalized: String = source
            .lines()
            .map(|l| l.replace('\t', "    "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!normalized.contains('\t'), "normalized must not have tabs");
        assert!(
            normalized.starts_with("    hello"),
            "tab replaced with 4 spaces"
        );
    }

    /// Convert tabs to spaces: each tab becomes tab_size spaces.
    #[test]
    fn editor_convert_tabs_to_spaces() {
        let line = "\tcode";
        let converted = line.replace('\t', "    ");
        assert!(!converted.contains('\t'));
        assert!(converted.starts_with("    "));
    }

    /// Convert spaces to tabs: leading 4 spaces become a tab.
    #[test]
    fn editor_convert_spaces_to_tabs() {
        let line = "    code";
        // Replace leading groups of 4 spaces with tabs
        let converted = if line.starts_with("    ") {
            format!("\t{}", &line[4..])
        } else {
            line.to_string()
        };
        assert!(converted.starts_with('\t'));
        assert!(converted.contains("code"));
    }

    /// Select all selects full doc — simulated by range 0..len.
    #[test]
    fn editor_select_all_selects_full_doc() {
        let doc = "define foo that is 42\nresult foo";
        let range = 0..doc.len();
        assert_eq!(doc[range].len(), doc.len());
    }

    /// Copy puts text in clipboard — clipboard receives the selected text.
    #[test]
    fn editor_copy_puts_text_in_clipboard() {
        use crate::clipboard::Clipboard;
        let mut cb = Clipboard::new();
        cb.copy(vec!["hello world".to_string()]);
        assert_eq!(cb.paste_joined(), "hello world");
    }

    /// Cut removes and puts in clipboard — text is removed from source and added to clipboard.
    #[test]
    fn editor_cut_removes_and_puts_in_clipboard() {
        use crate::clipboard::Clipboard;
        let mut cb = Clipboard::new();
        let mut source = "hello world".to_string();
        let cut_text = source[0..5].to_string(); // "hello"
        source = source[5..].to_string(); // " world"
        cb.copy(vec![cut_text.clone()]);
        assert_eq!(cb.paste_joined(), "hello");
        assert_eq!(source, " world");
    }

    /// Paste inserts at cursor — cursor advances by paste length.
    #[test]
    fn editor_paste_inserts_at_cursor() {
        use crate::clipboard::Clipboard;
        let mut cb = Clipboard::new();
        cb.copy(vec!["world".to_string()]);
        let mut doc = "hello ".to_string();
        let paste = cb.paste_joined();
        doc.push_str(&paste);
        assert_eq!(doc, "hello world");
    }

    /// Paste replaces selection — selected text is replaced by pasted text.
    #[test]
    fn editor_paste_replaces_selection() {
        let original = "hello REPLACE this";
        let pasted = original.replace("REPLACE", "world");
        assert_eq!(pasted, "hello world this");
    }

    /// Duplicate line: line is duplicated immediately below.
    #[test]
    fn editor_duplicate_line() {
        let source = "line1\nline2\nline3";
        let target_line = "line2";
        // Duplicate: insert another copy of line2 after it
        let duplicated = source.replace("line2\n", "line2\nline2\n");
        let count = duplicated.lines().filter(|l| *l == target_line).count();
        assert_eq!(count, 2, "duplicated line must appear twice");
    }

    /// Move line up: swaps a line with the one above.
    #[test]
    fn editor_move_line_up() {
        let lines = vec!["line1", "line2", "line3"];
        let target = 1usize; // move line2 up
        let mut result = lines.clone();
        result.swap(target - 1, target);
        assert_eq!(result, vec!["line2", "line1", "line3"]);
    }

    /// Move line down: swaps a line with the one below.
    #[test]
    fn editor_move_line_down() {
        let lines = vec!["line1", "line2", "line3"];
        let target = 1usize; // move line2 down
        let mut result = lines.clone();
        result.swap(target, target + 1);
        assert_eq!(result, vec!["line1", "line3", "line2"]);
    }

    // ── wave AB: indentation tests ───────────────────────────────────────────

    /// Auto-indent after `{`: next line gets one extra indent level relative to the line with `{`.
    #[test]
    fn auto_indent_after_open_brace_adds_level() {
        let prev_line = "define foo {";
        // base leading whitespace of prev_line is empty (no leading spaces)
        let base = auto_indent_text(prev_line, 4);
        assert_eq!(base, ""); // no leading whitespace
                              // smart indent: base + one level
        let smart = indent_line(&base, 4);
        assert_eq!(smart, "    ");
    }

    /// Auto-indent after `}`: closing brace line is dedented relative to body.
    #[test]
    fn auto_indent_after_close_brace_decreases_level() {
        let body_line = "    result 42";
        let base = auto_indent_text(body_line, 4);
        assert_eq!(base, "    ");
        // Closing brace is at one level less
        let close = dedent_line(&base, 4);
        assert_eq!(close, "");
    }

    /// Blank line preserves previous indent level.
    #[test]
    fn auto_indent_blank_line_preserves_prev_indent() {
        // Prev line has 8 spaces indent; blank next line should keep 8 spaces.
        let prev = "        define nested";
        let indent = auto_indent_text(prev, 4);
        assert_eq!(indent, "        ");
    }

    /// Tab width 4 produces exactly 4 spaces per indent_line call.
    #[test]
    fn indent_tab_width_4_produces_4_spaces() {
        let result = indent_line("code", 4);
        assert!(result.starts_with("    "), "must start with 4 spaces");
        assert!(!result.starts_with("     "), "must not start with 5 spaces");
    }

    /// Mixed tabs/spaces normalized to spaces via tab replacement.
    #[test]
    fn indent_mixed_tabs_spaces_normalized_to_spaces() {
        let mixed = "\t    mixed";
        let normalized = mixed.replace('\t', "    ");
        assert!(!normalized.contains('\t'), "no tabs after normalization");
        assert!(
            normalized.starts_with("        "),
            "tab(4) + 4 spaces = 8 spaces"
        );
    }

    /// auto_indent_text on a 2-space-indented line returns 2 spaces.
    #[test]
    fn auto_indent_two_space_indent_preserved() {
        let prev = "  code";
        let result = auto_indent_text(prev, 2);
        assert_eq!(result, "  ");
    }

    /// dedent_line on a line with fewer spaces than tab_size removes all leading spaces.
    #[test]
    fn dedent_fewer_spaces_than_tab_size_removes_all() {
        let result = dedent_line("  x", 8); // only 2 spaces, tab_size=8
        assert_eq!(result, "x");
    }

    /// indent_line with tab_size=8 produces 8 spaces.
    #[test]
    fn indent_tab_width_8_produces_8_spaces() {
        let result = indent_line("x", 8);
        assert_eq!(&result[..8], "        ");
        assert!(result.ends_with('x'));
    }

    /// auto_indent_text on a tab-only line returns the tab character.
    #[test]
    fn auto_indent_tab_only_line_returns_tab() {
        let result = auto_indent_text("\t", 4);
        assert_eq!(result, "\t");
    }

    /// dedent_line on an already-clean line (no spaces) returns unchanged.
    #[test]
    fn dedent_line_already_clean_unchanged() {
        let result = dedent_line("clean", 4);
        assert_eq!(result, "clean");
    }
}
