//! Sync protocol types for collaborative session message passing.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncMessageKind {
    Hello,
    Delta,
    Ack,
    Conflict,
    Bye,
}

impl SyncMessageKind {
    pub fn is_data(&self) -> bool {
        matches!(self, SyncMessageKind::Delta | SyncMessageKind::Conflict)
    }

    pub fn message_code(&self) -> u8 {
        match self {
            SyncMessageKind::Hello    => 0,
            SyncMessageKind::Delta    => 1,
            SyncMessageKind::Ack      => 2,
            SyncMessageKind::Conflict => 3,
            SyncMessageKind::Bye      => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMessage {
    pub kind: SyncMessageKind,
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub sender_id: u64,
}

impl SyncMessage {
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }

    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Syncing,
    Conflict,
    Disconnected,
}

impl SyncState {
    pub fn is_active(&self) -> bool {
        matches!(self, SyncState::Syncing)
    }

    pub fn can_send(&self) -> bool {
        matches!(self, SyncState::Idle | SyncState::Syncing)
    }
}

#[derive(Debug, Clone)]
pub struct SyncSession {
    pub session_id: u64,
    pub peer_id: u64,
    pub state: SyncState,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl SyncSession {
    pub fn record_sent(&mut self) {
        self.messages_sent += 1;
    }

    pub fn record_received(&mut self) {
        self.messages_received += 1;
    }

    pub fn set_state(&mut self, s: SyncState) {
        self.state = s;
    }

    pub fn total_messages(&self) -> u64 {
        self.messages_sent + self.messages_received
    }
}

#[derive(Debug, Clone)]
pub struct SyncProtocol {
    pub sessions: Vec<SyncSession>,
}

impl SyncProtocol {
    pub fn add_session(&mut self, s: SyncSession) {
        self.sessions.push(s);
    }

    pub fn active_sessions(&self) -> Vec<&SyncSession> {
        self.sessions.iter().filter(|s| s.state.is_active()).collect()
    }

    pub fn find_session(&self, session_id: u64) -> Option<&SyncSession> {
        self.sessions.iter().find(|s| s.session_id == session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_kind_is_data() {
        assert!(SyncMessageKind::Delta.is_data());
        assert!(SyncMessageKind::Conflict.is_data());
        assert!(!SyncMessageKind::Hello.is_data());
        assert!(!SyncMessageKind::Ack.is_data());
        assert!(!SyncMessageKind::Bye.is_data());
    }

    #[test]
    fn msg_kind_message_code() {
        assert_eq!(SyncMessageKind::Hello.message_code(), 0);
        assert_eq!(SyncMessageKind::Delta.message_code(), 1);
        assert_eq!(SyncMessageKind::Ack.message_code(), 2);
        assert_eq!(SyncMessageKind::Conflict.message_code(), 3);
        assert_eq!(SyncMessageKind::Bye.message_code(), 4);
    }

    #[test]
    fn message_payload_size() {
        let msg = SyncMessage {
            kind: SyncMessageKind::Delta,
            sequence: 1,
            payload: vec![1, 2, 3],
            sender_id: 42,
        };
        assert_eq!(msg.payload_size(), 3);
    }

    #[test]
    fn message_is_empty() {
        let empty = SyncMessage {
            kind: SyncMessageKind::Ack,
            sequence: 0,
            payload: vec![],
            sender_id: 1,
        };
        assert!(empty.is_empty());

        let non_empty = SyncMessage {
            kind: SyncMessageKind::Delta,
            sequence: 1,
            payload: vec![0xff],
            sender_id: 1,
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn state_is_active() {
        assert!(SyncState::Syncing.is_active());
        assert!(!SyncState::Idle.is_active());
        assert!(!SyncState::Conflict.is_active());
        assert!(!SyncState::Disconnected.is_active());
    }

    #[test]
    fn state_can_send() {
        assert!(SyncState::Idle.can_send());
        assert!(SyncState::Syncing.can_send());
        assert!(!SyncState::Conflict.can_send());
        assert!(!SyncState::Disconnected.can_send());
    }

    #[test]
    fn session_record_sent_and_total() {
        let mut session = SyncSession {
            session_id: 1,
            peer_id: 10,
            state: SyncState::Syncing,
            messages_sent: 0,
            messages_received: 3,
        };
        session.record_sent();
        session.record_sent();
        assert_eq!(session.messages_sent, 2);
        assert_eq!(session.total_messages(), 5);
    }

    #[test]
    fn session_set_state() {
        let mut session = SyncSession {
            session_id: 2,
            peer_id: 20,
            state: SyncState::Idle,
            messages_sent: 0,
            messages_received: 0,
        };
        assert!(!session.state.is_active());
        session.set_state(SyncState::Syncing);
        assert!(session.state.is_active());
    }

    #[test]
    fn protocol_active_sessions_count() {
        let mut proto = SyncProtocol { sessions: vec![] };
        proto.add_session(SyncSession {
            session_id: 1,
            peer_id: 1,
            state: SyncState::Syncing,
            messages_sent: 0,
            messages_received: 0,
        });
        proto.add_session(SyncSession {
            session_id: 2,
            peer_id: 2,
            state: SyncState::Idle,
            messages_sent: 0,
            messages_received: 0,
        });
        proto.add_session(SyncSession {
            session_id: 3,
            peer_id: 3,
            state: SyncState::Syncing,
            messages_sent: 0,
            messages_received: 0,
        });
        assert_eq!(proto.active_sessions().len(), 2);
        assert!(proto.find_session(2).is_some());
        assert!(proto.find_session(99).is_none());
    }
}
