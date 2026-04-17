//! Editor command dispatch.
//!
//! Commands take a `CommandContext` (mutable references to the buffer + selections)
//! and perform an edit or cursor motion.  They are looked up by stable id
//! strings so keybinding and command-palette can share the same registry.
#![deny(unsafe_code)]

use std::collections::HashMap;

pub type CommandId = &'static str;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("unknown command id '{0}'")]
    Unknown(String),
    #[error("command '{0}' failed: {1}")]
    #[allow(dead_code)]
    Failed(String, String),
}

pub struct CommandContext {
    /// Placeholder: in production this would be `&mut Buffer` + `&mut SelectionsCollection`.
    /// We store raw cursor positions + source text so commands can be tested in isolation.
    pub source: String,
    pub cursor_offset: usize,
}

impl CommandContext {
    pub fn new(source: impl Into<String>, cursor_offset: usize) -> Self {
        Self { source: source.into(), cursor_offset }
    }
}

type Handler = fn(&mut CommandContext) -> Result<(), CommandError>;

pub struct CommandRegistry {
    handlers: HashMap<CommandId, Handler>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register(&mut self, id: CommandId, handler: Handler) {
        self.handlers.insert(id, handler);
    }

    pub fn execute(&self, id: &str, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let handler = self.handlers.get(id).ok_or_else(|| CommandError::Unknown(id.to_string()))?;
        handler(ctx)
    }

    /// Seed with the default editor commands.
    pub fn with_defaults(mut self) -> Self {
        self.register("editor.move_left", cmd_move_left);
        self.register("editor.move_right", cmd_move_right);
        self.register("editor.move_home", cmd_move_home);
        self.register("editor.move_end", cmd_move_end);
        self.register("editor.delete_forward", cmd_delete_forward);
        self.register("editor.delete_backward", cmd_delete_backward);
        self.register("editor.insert_newline", cmd_insert_newline);
        self
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn all_ids(&self) -> Vec<CommandId> {
        self.handlers.keys().copied().collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Built-in handlers ───────────────────────────────────────────────────────

fn cmd_move_left(ctx: &mut CommandContext) -> Result<(), CommandError> {
    if ctx.cursor_offset > 0 {
        let mut i = ctx.cursor_offset - 1;
        while i > 0 && !ctx.source.is_char_boundary(i) {
            i -= 1;
        }
        ctx.cursor_offset = i;
    }
    Ok(())
}

fn cmd_move_right(ctx: &mut CommandContext) -> Result<(), CommandError> {
    let len = ctx.source.len();
    if ctx.cursor_offset < len {
        let mut i = ctx.cursor_offset + 1;
        while i < len && !ctx.source.is_char_boundary(i) {
            i += 1;
        }
        ctx.cursor_offset = i;
    }
    Ok(())
}

fn cmd_move_home(ctx: &mut CommandContext) -> Result<(), CommandError> {
    let before = &ctx.source[..ctx.cursor_offset];
    match before.rfind('\n') {
        Some(idx) => ctx.cursor_offset = idx + 1,
        None => ctx.cursor_offset = 0,
    }
    Ok(())
}

fn cmd_move_end(ctx: &mut CommandContext) -> Result<(), CommandError> {
    let after = &ctx.source[ctx.cursor_offset..];
    match after.find('\n') {
        Some(idx) => ctx.cursor_offset += idx,
        None => ctx.cursor_offset = ctx.source.len(),
    }
    Ok(())
}

fn cmd_delete_forward(ctx: &mut CommandContext) -> Result<(), CommandError> {
    if ctx.cursor_offset < ctx.source.len() {
        let mut end = ctx.cursor_offset + 1;
        while end < ctx.source.len() && !ctx.source.is_char_boundary(end) {
            end += 1;
        }
        ctx.source.drain(ctx.cursor_offset..end);
    }
    Ok(())
}

fn cmd_delete_backward(ctx: &mut CommandContext) -> Result<(), CommandError> {
    if ctx.cursor_offset > 0 {
        let mut start = ctx.cursor_offset - 1;
        while start > 0 && !ctx.source.is_char_boundary(start) {
            start -= 1;
        }
        ctx.source.drain(start..ctx.cursor_offset);
        ctx.cursor_offset = start;
    }
    Ok(())
}

fn cmd_insert_newline(ctx: &mut CommandContext) -> Result<(), CommandError> {
    ctx.source.insert(ctx.cursor_offset, '\n');
    ctx.cursor_offset += 1;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::new().with_defaults()
    }

    #[test]
    fn new_registry_is_empty() {
        let r = CommandRegistry::new();
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn with_defaults_registers_at_least_seven() {
        let r = reg();
        assert!(r.len() >= 7, "expected >=7 defaults, got {}", r.len());
    }

    #[test]
    fn all_ids_count_matches_len() {
        let r = reg();
        assert_eq!(r.all_ids().len(), r.len());
    }

    #[test]
    fn execute_unknown_returns_unknown_error() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 0);
        let err = r.execute("editor.nope", &mut ctx).unwrap_err();
        assert!(matches!(err, CommandError::Unknown(_)));
        assert!(err.to_string().contains("editor.nope"));
    }

    #[test]
    fn move_left_decrements_cursor() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 3);
        r.execute("editor.move_left", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 2);
    }

    #[test]
    fn move_left_saturates_at_zero() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 0);
        r.execute("editor.move_left", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 0);
    }

    #[test]
    fn move_right_increments_cursor_ascii() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 2);
        r.execute("editor.move_right", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 3);
    }

    #[test]
    fn move_right_saturates_at_len() {
        let r = reg();
        let src = "hi";
        let mut ctx = CommandContext::new(src, src.len());
        r.execute("editor.move_right", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, src.len());
    }

    #[test]
    fn move_right_skips_two_bytes_for_two_byte_char() {
        // 'é' is U+00E9, encoded as 2 bytes in UTF-8: [0xC3, 0xA9]
        let r = reg();
        let src = "é";
        assert_eq!(src.len(), 2, "sanity: é is 2 bytes");
        let mut ctx = CommandContext::new(src, 0);
        r.execute("editor.move_right", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 2);
    }

    #[test]
    fn move_home_finds_line_start_mid_line() {
        let r = reg();
        // "foo\nbar" — cursor at offset 6 ('a' in "bar")
        let mut ctx = CommandContext::new("foo\nbar", 6);
        r.execute("editor.move_home", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 4); // byte after '\n'
    }

    #[test]
    fn move_home_goes_to_zero_on_first_line() {
        let r = reg();
        let mut ctx = CommandContext::new("hello world", 5);
        r.execute("editor.move_home", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 0);
    }

    #[test]
    fn move_end_finds_newline() {
        let r = reg();
        // "foo\nbar" — cursor at 1 ('o'), end of first line is offset 3 (before '\n')
        let mut ctx = CommandContext::new("foo\nbar", 1);
        r.execute("editor.move_end", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, 3);
    }

    #[test]
    fn move_end_goes_to_eof_on_last_line() {
        let r = reg();
        let src = "hello";
        let mut ctx = CommandContext::new(src, 2);
        r.execute("editor.move_end", &mut ctx).unwrap();
        assert_eq!(ctx.cursor_offset, src.len());
    }

    #[test]
    fn delete_forward_removes_next_char() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 1);
        r.execute("editor.delete_forward", &mut ctx).unwrap();
        assert_eq!(ctx.source, "hllo");
        assert_eq!(ctx.cursor_offset, 1);
    }

    #[test]
    fn delete_forward_no_op_at_end() {
        let r = reg();
        let mut ctx = CommandContext::new("hi", 2);
        r.execute("editor.delete_forward", &mut ctx).unwrap();
        assert_eq!(ctx.source, "hi");
    }

    #[test]
    fn delete_backward_removes_prev_char_and_moves_cursor() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 3);
        r.execute("editor.delete_backward", &mut ctx).unwrap();
        assert_eq!(ctx.source, "helo");
        assert_eq!(ctx.cursor_offset, 2);
    }

    #[test]
    fn delete_backward_no_op_at_start() {
        let r = reg();
        let mut ctx = CommandContext::new("hi", 0);
        r.execute("editor.delete_backward", &mut ctx).unwrap();
        assert_eq!(ctx.source, "hi");
        assert_eq!(ctx.cursor_offset, 0);
    }

    #[test]
    fn insert_newline_inserts_and_advances_cursor() {
        let r = reg();
        let mut ctx = CommandContext::new("hello", 2);
        r.execute("editor.insert_newline", &mut ctx).unwrap();
        assert_eq!(ctx.source, "he\nllo");
        assert_eq!(ctx.cursor_offset, 3);
    }
}
