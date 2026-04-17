#![deny(unsafe_code)]
pub fn auto_indent_text(prev_line: &str, _tab_size: usize) -> String {
    let leading = prev_line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
    leading
}
pub fn indent_line(line: &str, tab_size: usize) -> String {
    format!("{}{}", " ".repeat(tab_size), line)
}
pub fn dedent_line(line: &str, tab_size: usize) -> String {
    let spaces_to_remove = line.chars().take_while(|c| *c == ' ').count().min(tab_size);
    line[spaces_to_remove..].to_string()
}
