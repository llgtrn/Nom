/// Role of a participant within a collaborative session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRole {
    Author,
    Reviewer,
    Observer,
}

/// A single participant in a collaborative session.
#[derive(Debug, Clone)]
pub struct CollabParticipant {
    pub id: String,
    pub role: SessionRole,
    pub joined_at: u64,
}

impl CollabParticipant {
    /// Create a new participant with the given identity, role, and join timestamp.
    pub fn new(id: impl Into<String>, role: SessionRole, ts: u64) -> Self {
        Self {
            id: id.into(),
            role,
            joined_at: ts,
        }
    }
}

/// A collaborative editing session that tracks its participants.
pub struct CollabSession {
    pub id: String,
    pub participants: Vec<CollabParticipant>,
    pub created_at: u64,
}

impl CollabSession {
    /// Create an empty session with the given id and creation timestamp.
    pub fn new(id: &str, ts: u64) -> Self {
        Self {
            id: id.to_owned(),
            participants: Vec::new(),
            created_at: ts,
        }
    }

    /// Add a participant to the session.
    pub fn join(&mut self, participant: CollabParticipant) {
        self.participants.push(participant);
    }

    /// Remove all participants whose id equals `participant_id`.
    pub fn leave(&mut self, participant_id: &str) {
        self.participants.retain(|p| p.id != participant_id);
    }

    /// Return the number of participants currently in the session.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Return `true` if a participant with `id` is present in the session.
    pub fn has_participant(&self, id: &str) -> bool {
        self.participants.iter().any(|p| p.id == id)
    }

    /// Return references to all participants whose role is `Author`.
    pub fn authors(&self) -> Vec<&CollabParticipant> {
        self.participants
            .iter()
            .filter(|p| p.role == SessionRole::Author)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new() {
        let s = CollabSession::new("s1", 1000);
        assert_eq!(s.id, "s1");
        assert_eq!(s.created_at, 1000);
        assert_eq!(s.participant_count(), 0);
    }

    #[test]
    fn join_and_count() {
        let mut s = CollabSession::new("s2", 0);
        s.join(CollabParticipant::new("alice", SessionRole::Author, 1));
        s.join(CollabParticipant::new("bob", SessionRole::Reviewer, 2));
        assert_eq!(s.participant_count(), 2);
    }

    #[test]
    fn leave() {
        let mut s = CollabSession::new("s3", 0);
        s.join(CollabParticipant::new("alice", SessionRole::Author, 1));
        s.join(CollabParticipant::new("bob", SessionRole::Observer, 2));
        s.leave("alice");
        assert_eq!(s.participant_count(), 1);
        assert!(!s.has_participant("alice"));
        assert!(s.has_participant("bob"));
    }

    #[test]
    fn has_participant() {
        let mut s = CollabSession::new("s4", 0);
        s.join(CollabParticipant::new("carol", SessionRole::Reviewer, 5));
        assert!(s.has_participant("carol"));
        assert!(!s.has_participant("dave"));
    }

    #[test]
    fn authors_only() {
        let mut s = CollabSession::new("s5", 0);
        s.join(CollabParticipant::new("a1", SessionRole::Author, 1));
        s.join(CollabParticipant::new("r1", SessionRole::Reviewer, 2));
        s.join(CollabParticipant::new("a2", SessionRole::Author, 3));
        s.join(CollabParticipant::new("o1", SessionRole::Observer, 4));
        let authors = s.authors();
        assert_eq!(authors.len(), 2);
        assert!(authors.iter().all(|p| p.role == SessionRole::Author));
    }

    #[test]
    fn participant_roles() {
        let p_author = CollabParticipant::new("x", SessionRole::Author, 0);
        let p_reviewer = CollabParticipant::new("y", SessionRole::Reviewer, 0);
        let p_observer = CollabParticipant::new("z", SessionRole::Observer, 0);
        assert_eq!(p_author.role, SessionRole::Author);
        assert_eq!(p_reviewer.role, SessionRole::Reviewer);
        assert_eq!(p_observer.role, SessionRole::Observer);
        assert_ne!(p_author.role, p_reviewer.role);
    }
}
