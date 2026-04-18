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

// ---------------------------------------------------------------------------
// Huly/CRDT-style collaborative presence awareness
// ---------------------------------------------------------------------------

/// Status of a user in the collaborative workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceUserStatus {
    Active,
    Idle,
    Away,
}

impl PresenceUserStatus {
    pub fn status_name(&self) -> &str {
        match self {
            PresenceUserStatus::Active => "active",
            PresenceUserStatus::Idle => "idle",
            PresenceUserStatus::Away => "away",
        }
    }

    /// Returns true if the user counts as online (Active or Idle).
    pub fn is_online(&self) -> bool {
        matches!(self, PresenceUserStatus::Active | PresenceUserStatus::Idle)
    }
}

/// A single user currently present in the workspace.
#[derive(Debug, Clone)]
pub struct PresenceUser {
    pub user_id: u64,
    pub display_name: String,
    pub status: PresenceUserStatus,
    pub cursor_x: f32,
    pub cursor_y: f32,
}

impl PresenceUser {
    pub fn new(user_id: u64, display_name: impl Into<String>) -> Self {
        Self {
            user_id,
            display_name: display_name.into(),
            status: PresenceUserStatus::Active,
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }

    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn set_status(&mut self, status: PresenceUserStatus) {
        self.status = status;
    }

    pub fn is_online(&self) -> bool {
        self.status.is_online()
    }
}

/// Tracks all users currently present in the workspace.
pub struct PresenceUserMap {
    pub users: std::collections::HashMap<u64, PresenceUser>,
}

impl PresenceUserMap {
    pub fn new() -> Self {
        Self {
            users: std::collections::HashMap::new(),
        }
    }

    pub fn join(&mut self, user: PresenceUser) {
        self.users.insert(user.user_id, user);
    }

    pub fn leave(&mut self, user_id: u64) {
        self.users.remove(&user_id);
    }

    pub fn update_cursor(&mut self, user_id: u64, x: f32, y: f32) {
        if let Some(user) = self.users.get_mut(&user_id) {
            user.update_cursor(x, y);
        }
    }

    /// Count of users that are online (Active or Idle).
    pub fn online_count(&self) -> usize {
        self.users.values().filter(|u| u.is_online()).count()
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    pub fn get_user(&self, user_id: u64) -> Option<&PresenceUser> {
        self.users.get(&user_id)
    }
}

impl Default for PresenceUserMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted when presence state changes.
#[derive(Debug, Clone)]
pub enum PresenceEvent {
    UserJoined(u64),
    UserLeft(u64),
    CursorMoved(u64, f32, f32),
    StatusChanged(u64),
}

impl PresenceEvent {
    pub fn event_type(&self) -> &str {
        match self {
            PresenceEvent::UserJoined(_) => "user_joined",
            PresenceEvent::UserLeft(_) => "user_left",
            PresenceEvent::CursorMoved(_, _, _) => "cursor_moved",
            PresenceEvent::StatusChanged(_) => "status_changed",
        }
    }
}

/// Event broadcast for presence changes.
pub struct PresenceBroadcast {
    pub events: Vec<PresenceEvent>,
}

impl PresenceBroadcast {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn emit(&mut self, event: PresenceEvent) {
        self.events.push(event);
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn latest(&self) -> Option<&PresenceEvent> {
        self.events.last()
    }
}

impl Default for PresenceBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod presence_tests {
    use super::*;

    #[test]
    fn presence_status_is_online_active() {
        assert!(PresenceUserStatus::Active.is_online());
    }

    #[test]
    fn presence_status_is_online_away_false() {
        assert!(!PresenceUserStatus::Away.is_online());
    }

    #[test]
    fn presence_user_update_cursor() {
        let mut user = PresenceUser::new(1, "alice");
        user.update_cursor(42.0, 99.5);
        assert_eq!(user.cursor_x, 42.0);
        assert_eq!(user.cursor_y, 99.5);
    }

    #[test]
    fn presence_user_map_join_and_count() {
        let mut map = PresenceUserMap::new();
        map.join(PresenceUser::new(1, "alice"));
        map.join(PresenceUser::new(2, "bob"));
        assert_eq!(map.user_count(), 2);
    }

    #[test]
    fn presence_user_map_leave_removes() {
        let mut map = PresenceUserMap::new();
        map.join(PresenceUser::new(1, "alice"));
        map.leave(1);
        assert_eq!(map.user_count(), 0);
        assert!(map.get_user(1).is_none());
    }

    #[test]
    fn presence_user_map_online_count() {
        let mut map = PresenceUserMap::new();
        map.join(PresenceUser::new(1, "alice")); // Active → online
        let mut idle = PresenceUser::new(2, "bob");
        idle.set_status(PresenceUserStatus::Idle); // Idle → online
        map.join(idle);
        let mut away = PresenceUser::new(3, "carol");
        away.set_status(PresenceUserStatus::Away); // Away → offline
        map.join(away);
        assert_eq!(map.online_count(), 2);
    }

    #[test]
    fn presence_event_event_type() {
        assert_eq!(PresenceEvent::UserJoined(1).event_type(), "user_joined");
        assert_eq!(PresenceEvent::UserLeft(1).event_type(), "user_left");
        assert_eq!(PresenceEvent::CursorMoved(1, 0.0, 0.0).event_type(), "cursor_moved");
        assert_eq!(PresenceEvent::StatusChanged(1).event_type(), "status_changed");
    }

    #[test]
    fn presence_broadcast_emit_and_count() {
        let mut bc = PresenceBroadcast::new();
        bc.emit(PresenceEvent::UserJoined(1));
        bc.emit(PresenceEvent::CursorMoved(1, 10.0, 20.0));
        assert_eq!(bc.event_count(), 2);
    }

    #[test]
    fn presence_broadcast_latest() {
        let mut bc = PresenceBroadcast::new();
        assert!(bc.latest().is_none());
        bc.emit(PresenceEvent::UserJoined(42));
        bc.emit(PresenceEvent::StatusChanged(42));
        match bc.latest() {
            Some(PresenceEvent::StatusChanged(id)) => assert_eq!(*id, 42),
            _ => panic!("expected StatusChanged"),
        }
    }
}
