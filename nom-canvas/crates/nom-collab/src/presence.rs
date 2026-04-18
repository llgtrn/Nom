// ---------------------------------------------------------------------------
// Presence — user status and cursor tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    Online,
    Away,
    Offline,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
    pub line: u32,
    pub col: u32,
    pub user_id: String,
}

impl CursorPosition {
    pub fn new(user_id: &str, line: u32, col: u32) -> Self {
        Self {
            line,
            col,
            user_id: user_id.to_string(),
        }
    }

    pub fn same_line_as(&self, other: &CursorPosition) -> bool {
        self.line == other.line
    }
}

pub struct PresenceMap {
    pub entries: Vec<(String, PresenceStatus)>,
    pub cursors: Vec<CursorPosition>,
}

impl PresenceMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursors: Vec::new(),
        }
    }

    pub fn set_status(&mut self, user_id: &str, status: PresenceStatus) {
        if let Some(entry) = self.entries.iter_mut().find(|(id, _)| id == user_id) {
            entry.1 = status;
        } else {
            self.entries.push((user_id.to_string(), status));
        }
    }

    pub fn get_status(&self, user_id: &str) -> Option<&PresenceStatus> {
        self.entries
            .iter()
            .find(|(id, _)| id == user_id)
            .map(|(_, status)| status)
    }

    pub fn set_cursor(&mut self, cursor: CursorPosition) {
        if let Some(existing) = self
            .cursors
            .iter_mut()
            .find(|c| c.user_id == cursor.user_id)
        {
            *existing = cursor;
        } else {
            self.cursors.push(cursor);
        }
    }

    pub fn online_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|(_, status)| *status == PresenceStatus::Online)
            .count()
    }

    pub fn cursors_on_line(&self, line: u32) -> Vec<&CursorPosition> {
        self.cursors.iter().filter(|c| c.line == line).collect()
    }
}

impl Default for PresenceMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_position_new() {
        let c = CursorPosition::new("alice", 3, 7);
        assert_eq!(c.user_id, "alice");
        assert_eq!(c.line, 3);
        assert_eq!(c.col, 7);
    }

    #[test]
    fn cursor_same_line() {
        let a = CursorPosition::new("alice", 5, 0);
        let b = CursorPosition::new("bob", 5, 10);
        let c = CursorPosition::new("carol", 6, 0);
        assert!(a.same_line_as(&b));
        assert!(!a.same_line_as(&c));
    }

    #[test]
    fn presence_set_status() {
        let mut map = PresenceMap::new();
        map.set_status("alice", PresenceStatus::Online);
        map.set_status("alice", PresenceStatus::Away);
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.entries[0].1, PresenceStatus::Away);
    }

    #[test]
    fn presence_get_status() {
        let mut map = PresenceMap::new();
        map.set_status("bob", PresenceStatus::Busy);
        assert_eq!(map.get_status("bob"), Some(&PresenceStatus::Busy));
        assert_eq!(map.get_status("unknown"), None);
    }

    #[test]
    fn online_count() {
        let mut map = PresenceMap::new();
        map.set_status("alice", PresenceStatus::Online);
        map.set_status("bob", PresenceStatus::Offline);
        map.set_status("carol", PresenceStatus::Online);
        map.set_status("dave", PresenceStatus::Away);
        assert_eq!(map.online_count(), 2);
    }

    #[test]
    fn cursors_on_line() {
        let mut map = PresenceMap::new();
        map.set_cursor(CursorPosition::new("alice", 10, 0));
        map.set_cursor(CursorPosition::new("bob", 10, 5));
        map.set_cursor(CursorPosition::new("carol", 11, 0));
        let on_10 = map.cursors_on_line(10);
        assert_eq!(on_10.len(), 2);
        let on_11 = map.cursors_on_line(11);
        assert_eq!(on_11.len(), 1);
        let on_99 = map.cursors_on_line(99);
        assert_eq!(on_99.len(), 0);
    }
}
