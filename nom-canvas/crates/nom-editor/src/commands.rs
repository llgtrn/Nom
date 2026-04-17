#![deny(unsafe_code)]
use std::collections::HashMap;

pub struct CommandId(pub &'static str);
pub type CommandFn = Box<dyn Fn() + Send + Sync>;
pub struct CommandRegistry { commands: HashMap<&'static str, CommandFn> }
impl CommandRegistry {
    pub fn new() -> Self { Self { commands: HashMap::new() } }
    pub fn register(&mut self, id: &'static str, f: impl Fn() + Send + Sync + 'static) {
        self.commands.insert(id, Box::new(f));
    }
    pub fn dispatch(&self, id: &str) -> bool {
        if let Some(f) = self.commands.get(id) { f(); true } else { false }
    }
    pub fn command_ids(&self) -> Vec<&&'static str> { self.commands.keys().collect() }
}
impl Default for CommandRegistry { fn default() -> Self { Self::new() } }
