// Integration tests for nom_editor::indent — mirrors the unit tests in src/indent.rs.
// The module must be declared in lib.rs to be reachable; until then this file
// exercises the public surface via a re-include so the test suite can run standalone.

// Inline the module so tests compile without touching lib.rs.
#[path = "../src/indent.rs"]
mod indent;

use indent::*;

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
