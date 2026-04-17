//! Indent / dedent helpers + indentation-style detection.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentKind { Spaces, Tabs }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndentStyle {
    pub kind: IndentKind,
    pub width: u32,
}

impl IndentStyle {
    pub const SPACES_4: Self = Self { kind: IndentKind::Spaces, width: 4 };
    pub const SPACES_2: Self = Self { kind: IndentKind::Spaces, width: 2 };
    pub const TABS: Self = Self { kind: IndentKind::Tabs, width: 1 };

    /// Return the string a single level of indent produces.
    pub fn single_level(self) -> String {
        match self.kind {
            IndentKind::Spaces => " ".repeat(self.width as usize),
            IndentKind::Tabs => "\t".repeat(self.width.max(1) as usize),
        }
    }
}

impl Default for IndentStyle { fn default() -> Self { Self::SPACES_4 } }

/// Infer indent style from source.  Picks the most common leading-whitespace
/// pattern across non-blank lines.  Returns `SPACES_4` as fallback.
pub fn detect_style(source: &str) -> IndentStyle {
    let mut tab_lines = 0usize;
    let mut space2_lines = 0usize;
    let mut space4_lines = 0usize;
    for line in source.lines() {
        if line.trim().is_empty() { continue; }
        if line.starts_with('\t') { tab_lines += 1; continue; }
        let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
        if leading_spaces >= 4 && leading_spaces % 4 == 0 { space4_lines += 1; }
        else if leading_spaces >= 2 && leading_spaces % 2 == 0 { space2_lines += 1; }
    }
    if tab_lines >= space4_lines && tab_lines >= space2_lines && tab_lines > 0 { return IndentStyle::TABS; }
    if space2_lines > space4_lines { return IndentStyle::SPACES_2; }
    IndentStyle::SPACES_4
}

/// Indent each line in [first_line..=last_line] by one level.
pub fn indent_lines(source: &str, first_line: u32, last_line: u32, style: IndentStyle) -> String {
    let indent = style.single_level();
    let mut out = String::with_capacity(source.len() + (last_line - first_line + 1) as usize * indent.len());
    for (i, line) in source.lines().enumerate() {
        let in_range = i as u32 >= first_line && i as u32 <= last_line;
        if in_range && !line.is_empty() { out.push_str(&indent); }
        out.push_str(line);
        out.push('\n');
    }
    // Rust's lines() strips the trailing newline; restore if the source didn't have one.
    if !source.ends_with('\n') { out.pop(); }
    out
}

/// Dedent each line in [first_line..=last_line] by up to one level.  Leaves
/// lines alone that don't start with the indent string.
pub fn dedent_lines(source: &str, first_line: u32, last_line: u32, style: IndentStyle) -> String {
    let indent = style.single_level();
    let mut out = String::with_capacity(source.len());
    for (i, line) in source.lines().enumerate() {
        let in_range = i as u32 >= first_line && i as u32 <= last_line;
        let stripped = if in_range {
            line.strip_prefix(&indent).unwrap_or(line)
        } else {
            line
        };
        out.push_str(stripped);
        out.push('\n');
    }
    if !source.ends_with('\n') { out.pop(); }
    out
}

/// Return the current indent depth of a line (in levels, given style).
pub fn depth_of_line(line: &str, style: IndentStyle) -> u32 {
    let indent = style.single_level();
    let mut depth = 0u32;
    let mut remaining = line;
    while remaining.starts_with(&indent) {
        depth += 1;
        remaining = &remaining[indent.len()..];
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces4_single_level() {
        assert_eq!(IndentStyle::SPACES_4.single_level(), "    ");
    }

    #[test]
    fn spaces2_single_level() {
        assert_eq!(IndentStyle::SPACES_2.single_level(), "  ");
    }

    #[test]
    fn tabs_single_level() {
        assert_eq!(IndentStyle::TABS.single_level(), "\t");
    }

    #[test]
    fn default_is_spaces4() {
        assert_eq!(IndentStyle::default(), IndentStyle::SPACES_4);
    }

    #[test]
    fn detect_tabs() {
        assert_eq!(detect_style("\tfoo\n\tbar\n"), IndentStyle::TABS);
    }

    #[test]
    fn detect_spaces2() {
        assert_eq!(detect_style("  foo\n  bar\n"), IndentStyle::SPACES_2);
    }

    #[test]
    fn detect_spaces4() {
        assert_eq!(detect_style("    foo\n    bar\n"), IndentStyle::SPACES_4);
    }

    #[test]
    fn detect_empty_fallback() {
        assert_eq!(detect_style(""), IndentStyle::SPACES_4);
    }

    #[test]
    fn indent_lines_adds_prefix_to_target_only() {
        let src = "line0\nline1\nline2\n";
        let result = indent_lines(src, 1, 1, IndentStyle::SPACES_4);
        assert_eq!(result, "line0\n    line1\nline2\n");
    }

    #[test]
    fn indent_lines_skips_empty_lines() {
        let src = "line0\n\nline2\n";
        let result = indent_lines(src, 0, 2, IndentStyle::SPACES_4);
        assert_eq!(result, "    line0\n\n    line2\n");
    }

    #[test]
    fn indent_lines_no_trailing_newline() {
        let src = "foo\nbar";
        let result = indent_lines(src, 0, 1, IndentStyle::SPACES_2);
        assert_eq!(result, "  foo\n  bar");
    }

    #[test]
    fn dedent_removes_prefix() {
        let src = "    foo\n    bar\n";
        let result = dedent_lines(src, 0, 1, IndentStyle::SPACES_4);
        assert_eq!(result, "foo\nbar\n");
    }

    #[test]
    fn dedent_leaves_under_indented_unchanged() {
        let src = "foo\n    bar\n";
        let result = dedent_lines(src, 0, 1, IndentStyle::SPACES_4);
        assert_eq!(result, "foo\nbar\n");
    }

    #[test]
    fn dedent_range_exclusive_of_other_lines() {
        let src = "    line0\n    line1\n    line2\n";
        let result = dedent_lines(src, 1, 1, IndentStyle::SPACES_4);
        assert_eq!(result, "    line0\nline1\n    line2\n");
    }

    #[test]
    fn depth_spaces4_one_level() {
        assert_eq!(depth_of_line("    foo", IndentStyle::SPACES_4), 1);
    }

    #[test]
    fn depth_spaces4_two_levels() {
        assert_eq!(depth_of_line("        foo", IndentStyle::SPACES_4), 2);
    }

    #[test]
    fn depth_tabs_counts() {
        assert_eq!(depth_of_line("\t\tfoo", IndentStyle::TABS), 2);
    }

    #[test]
    fn depth_unindented_zero() {
        assert_eq!(depth_of_line("foo", IndentStyle::SPACES_4), 0);
    }
}
