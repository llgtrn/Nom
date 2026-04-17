#![deny(unsafe_code)]
pub struct Clipboard {
    contents: Vec<String>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
        }
    }
    pub fn copy(&mut self, texts: Vec<String>) {
        self.contents = texts;
    }
    pub fn paste(&self) -> Vec<String> {
        self.contents.clone()
    }
    pub fn paste_joined(&self) -> String {
        self.contents.join("\n")
    }
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }
}
impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}
