#![deny(unsafe_code)]
use std::collections::HashMap;

pub struct CommandId(pub &'static str);

pub struct EditorContext {
    pub cursor: usize,
    pub buffer_len: usize,
}

pub type CommandFn = Box<dyn Fn(&mut EditorContext) + Send + Sync>;

pub struct CommandRegistry { commands: HashMap<&'static str, CommandFn> }
impl CommandRegistry {
    pub fn new() -> Self { Self { commands: HashMap::new() } }
    pub fn register(&mut self, id: &'static str, f: impl Fn(&mut EditorContext) + Send + Sync + 'static) {
        self.commands.insert(id, Box::new(f));
    }
    pub fn execute(&self, id: &str, ctx: &mut EditorContext) -> bool {
        if let Some(f) = self.commands.get(id) { f(ctx); true } else { false }
    }
    pub fn dispatch(&self, id: &str, ctx: &mut EditorContext) -> bool {
        self.execute(id, ctx)
    }
    pub fn command_ids(&self) -> Vec<&&'static str> { self.commands.keys().collect() }
    pub fn has_command(&self, id: &str) -> bool { self.commands.contains_key(id) }
}
impl Default for CommandRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn command_registry_dispatch_known() {
        let mut registry = CommandRegistry::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();
        registry.register("save", move |_ctx| { *called_clone.lock().unwrap() = true; });
        let mut ctx = EditorContext { cursor: 0, buffer_len: 10 };
        let result = registry.dispatch("save", &mut ctx);
        assert!(result);
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn command_registry_dispatch_unknown_returns_false() {
        let registry = CommandRegistry::new();
        let mut ctx = EditorContext { cursor: 0, buffer_len: 0 };
        assert!(!registry.dispatch("nonexistent", &mut ctx));
    }

    #[test]
    fn command_registry_lists_registered_ids() {
        let mut registry = CommandRegistry::new();
        registry.register("save", |_ctx| {});
        registry.register("open", |_ctx| {});
        let ids = registry.command_ids();
        assert_eq!(ids.len(), 2);
        let id_strs: Vec<&str> = ids.iter().map(|&&s| s).collect();
        assert!(id_strs.contains(&"save"));
        assert!(id_strs.contains(&"open"));
    }

    #[test]
    fn command_registry_has_command() {
        let mut registry = CommandRegistry::new();
        registry.register("undo", |_ctx| {});
        assert!(registry.has_command("undo"));
        assert!(!registry.has_command("redo"));
    }

    #[test]
    fn command_receives_context() {
        let mut registry = CommandRegistry::new();
        registry.register("move_end", |ctx| { ctx.cursor = ctx.buffer_len; });
        let mut ctx = EditorContext { cursor: 3, buffer_len: 20 };
        registry.execute("move_end", &mut ctx);
        assert_eq!(ctx.cursor, 20);
    }
}
