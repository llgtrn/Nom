use std::collections::HashMap;

/// Online/offline status of a peer in a collaborative session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceStatus {
    Active,
    Idle,
    Away,
    Offline,
}

impl PresenceStatus {
    /// Returns true for any variant that represents an online peer.
    pub fn is_online(&self) -> bool {
        matches!(self, Self::Active | Self::Idle | Self::Away)
    }

    /// Returns a hex color string suitable for UI display.
    pub fn display_color(&self) -> &'static str {
        match self {
            Self::Active  => "#22c55e",
            Self::Idle    => "#eab308",
            Self::Away    => "#f97316",
            Self::Offline => "#6b7280",
        }
    }
}

/// Newtype wrapper around a u32 peer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId(pub u32);

impl PeerId {
    /// A peer id of 0 is reserved and treated as invalid.
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Returns a stable string key for use in maps or logs.
    pub fn peer_key(&self) -> String {
        format!("peer:{}", self.0)
    }
}

/// A snapshot of one peer's presence state, including cursor position.
#[derive(Debug, Clone)]
pub struct PresenceEntry {
    pub peer: PeerId,
    pub name: String,
    pub status: PresenceStatus,
    pub cursor_x: f32,
    pub cursor_y: f32,
}

impl PresenceEntry {
    /// Returns true when the peer's status is exactly `Active`.
    pub fn is_active(&self) -> bool {
        self.status == PresenceStatus::Active
    }

    /// Human-readable summary: "Alice (#22c55e) @ (120,340)".
    pub fn display(&self) -> String {
        format!(
            "{} ({}) @ ({:.0},{:.0})",
            self.name,
            self.status.display_color(),
            self.cursor_x,
            self.cursor_y,
        )
    }
}

/// Registry of all known peers indexed by their numeric id.
pub struct PresenceMap {
    pub entries: HashMap<u32, PresenceEntry>,
}

impl PresenceMap {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert or replace the entry for `entry.peer`.
    pub fn upsert(&mut self, entry: PresenceEntry) {
        self.entries.insert(entry.peer.0, entry);
    }

    /// Remove the entry for `peer` if it exists.
    pub fn remove(&mut self, peer: &PeerId) {
        self.entries.remove(&peer.0);
    }

    /// Returns all entries whose status satisfies `is_online()`.
    pub fn online_peers(&self) -> Vec<&PresenceEntry> {
        self.entries
            .values()
            .filter(|e| e.status.is_online())
            .collect()
    }

    /// Total number of tracked peers (online or not).
    pub fn peer_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for PresenceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Accumulates broadcast counts driven by presence snapshots.
pub struct PresenceBroadcaster {
    sent: u32,
}

impl PresenceBroadcaster {
    pub fn new() -> Self {
        Self { sent: 0 }
    }

    /// Increments `sent` by the number of online peers and returns the new total.
    pub fn broadcast(&mut self, map: &PresenceMap) -> u32 {
        self.sent += map.online_peers().len() as u32;
        self.sent
    }

    pub fn total_sent(&self) -> u32 {
        self.sent
    }
}

impl Default for PresenceBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: u32, name: &str, status: PresenceStatus, x: f32, y: f32) -> PresenceEntry {
        PresenceEntry {
            peer: PeerId(id),
            name: name.to_string(),
            status,
            cursor_x: x,
            cursor_y: y,
        }
    }

    #[test]
    fn presence_status_is_online_all_variants() {
        assert!(PresenceStatus::Active.is_online());
        assert!(PresenceStatus::Idle.is_online());
        assert!(PresenceStatus::Away.is_online());
        assert!(!PresenceStatus::Offline.is_online());
    }

    #[test]
    fn presence_status_display_color_active() {
        assert_eq!(PresenceStatus::Active.display_color(), "#22c55e");
    }

    #[test]
    fn peer_id_is_valid() {
        assert!(!PeerId(0).is_valid());
        assert!(PeerId(1).is_valid());
        assert!(PeerId(u32::MAX).is_valid());
    }

    #[test]
    fn presence_entry_is_active() {
        let active = make_entry(1, "Alice", PresenceStatus::Active, 0.0, 0.0);
        let idle   = make_entry(2, "Bob",   PresenceStatus::Idle,   0.0, 0.0);
        assert!(active.is_active());
        assert!(!idle.is_active());
    }

    #[test]
    fn presence_entry_display_format() {
        let entry = make_entry(1, "Alice", PresenceStatus::Active, 120.0, 340.0);
        assert_eq!(entry.display(), "Alice (#22c55e) @ (120,340)");
    }

    #[test]
    fn presence_map_upsert_and_peer_count() {
        let mut map = PresenceMap::new();
        assert_eq!(map.peer_count(), 0);
        map.upsert(make_entry(1, "Alice", PresenceStatus::Active, 0.0, 0.0));
        assert_eq!(map.peer_count(), 1);
        // upsert same id replaces, count stays 1
        map.upsert(make_entry(1, "Alice2", PresenceStatus::Idle, 5.0, 5.0));
        assert_eq!(map.peer_count(), 1);
        map.upsert(make_entry(2, "Bob", PresenceStatus::Active, 0.0, 0.0));
        assert_eq!(map.peer_count(), 2);
    }

    #[test]
    fn presence_map_online_peers_filter() {
        let mut map = PresenceMap::new();
        map.upsert(make_entry(1, "Alice",   PresenceStatus::Active,  0.0, 0.0));
        map.upsert(make_entry(2, "Bob",     PresenceStatus::Offline, 0.0, 0.0));
        map.upsert(make_entry(3, "Charlie", PresenceStatus::Away,    0.0, 0.0));
        let online = map.online_peers();
        assert_eq!(online.len(), 2);
    }

    #[test]
    fn presence_map_remove() {
        let mut map = PresenceMap::new();
        map.upsert(make_entry(1, "Alice", PresenceStatus::Active, 0.0, 0.0));
        map.upsert(make_entry(2, "Bob",   PresenceStatus::Active, 0.0, 0.0));
        assert_eq!(map.peer_count(), 2);
        map.remove(&PeerId(1));
        assert_eq!(map.peer_count(), 1);
        assert!(!map.entries.contains_key(&1));
    }

    #[test]
    fn presence_broadcaster_broadcast_accumulates() {
        let mut map = PresenceMap::new();
        map.upsert(make_entry(1, "Alice", PresenceStatus::Active,  0.0, 0.0));
        map.upsert(make_entry(2, "Bob",   PresenceStatus::Idle,    0.0, 0.0));
        map.upsert(make_entry(3, "Carol", PresenceStatus::Offline, 0.0, 0.0));

        let mut bc = PresenceBroadcaster::new();
        assert_eq!(bc.total_sent(), 0);

        // First broadcast: 2 online peers → sent = 2
        let after_first = bc.broadcast(&map);
        assert_eq!(after_first, 2);
        assert_eq!(bc.total_sent(), 2);

        // Second broadcast: still 2 online peers → sent = 4
        let after_second = bc.broadcast(&map);
        assert_eq!(after_second, 4);
        assert_eq!(bc.total_sent(), 4);
    }
}
