/// Audit log types for NomCanvas telemetry.

#[derive(Debug, Clone, PartialEq)]
pub enum AuditCategory {
    Security,
    DataAccess,
    Configuration,
    UserAction,
    SystemOp,
}

impl AuditCategory {
    pub fn requires_retention(&self) -> bool {
        matches!(self, AuditCategory::Security | AuditCategory::DataAccess)
    }

    pub fn category_code(&self) -> &'static str {
        match self {
            AuditCategory::Security => "SEC",
            AuditCategory::DataAccess => "DA",
            AuditCategory::Configuration => "CFG",
            AuditCategory::UserAction => "UA",
            AuditCategory::SystemOp => "SYS",
        }
    }
}

pub struct AuditEvent {
    pub id: u64,
    pub category: AuditCategory,
    pub actor: String,
    pub action: String,
    pub timestamp_ms: u64,
    pub success: bool,
}

impl AuditEvent {
    pub fn summary(&self) -> String {
        let status = if self.success { "OK" } else { "FAIL" };
        format!(
            "[{}] {}: {} ({})",
            self.category.category_code(),
            self.actor,
            self.action,
            status
        )
    }

    pub fn is_security_relevant(&self) -> bool {
        self.category.requires_retention()
    }
}

pub struct AuditFilter {
    pub category: Option<AuditCategory>,
    pub actor: Option<String>,
    pub success_only: bool,
}

impl AuditFilter {
    pub fn matches(&self, e: &AuditEvent) -> bool {
        if let Some(ref cat) = self.category {
            if &e.category != cat {
                return false;
            }
        }
        if let Some(ref actor) = self.actor {
            if &e.actor != actor {
                return false;
            }
        }
        if self.success_only && !e.success {
            return false;
        }
        true
    }
}

pub struct AuditLog {
    pub events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, e: AuditEvent) {
        self.events.push(e);
    }

    pub fn query(&self, f: &AuditFilter) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| f.matches(e)).collect()
    }

    pub fn security_events(&self) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.is_security_relevant()).collect()
    }

    pub fn latest_n(&self, n: usize) -> Vec<&AuditEvent> {
        let len = self.events.len();
        if n >= len {
            self.events.iter().collect()
        } else {
            self.events[len - n..].iter().collect()
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AuditReporter {
    pub log: AuditLog,
}

impl AuditReporter {
    pub fn new() -> Self {
        Self { log: AuditLog::new() }
    }

    pub fn add(&mut self, e: AuditEvent) {
        self.log.record(e);
    }

    pub fn report_summary(&self) -> String {
        format!(
            "total:{} security:{}",
            self.log.events.len(),
            self.log.security_events().len()
        )
    }
}

impl Default for AuditReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod audit_log_tests {
    use super::*;

    fn make_event(id: u64, category: AuditCategory, actor: &str, action: &str, success: bool) -> AuditEvent {
        AuditEvent {
            id,
            category,
            actor: actor.to_string(),
            action: action.to_string(),
            timestamp_ms: 1000,
            success,
        }
    }

    #[test]
    fn category_requires_retention() {
        assert!(AuditCategory::Security.requires_retention());
        assert!(AuditCategory::DataAccess.requires_retention());
        assert!(!AuditCategory::Configuration.requires_retention());
        assert!(!AuditCategory::UserAction.requires_retention());
        assert!(!AuditCategory::SystemOp.requires_retention());
    }

    #[test]
    fn category_code_security_is_sec() {
        assert_eq!(AuditCategory::Security.category_code(), "SEC");
    }

    #[test]
    fn event_summary_format_ok() {
        let e = make_event(1, AuditCategory::UserAction, "alice", "login", true);
        assert_eq!(e.summary(), "[UA] alice: login (OK)");
    }

    #[test]
    fn event_summary_format_fail() {
        let e = make_event(2, AuditCategory::Security, "bob", "access_secret", false);
        assert_eq!(e.summary(), "[SEC] bob: access_secret (FAIL)");
    }

    #[test]
    fn event_is_security_relevant() {
        let sec = make_event(3, AuditCategory::Security, "a", "op", true);
        let da = make_event(4, AuditCategory::DataAccess, "a", "op", true);
        let cfg = make_event(5, AuditCategory::Configuration, "a", "op", true);
        assert!(sec.is_security_relevant());
        assert!(da.is_security_relevant());
        assert!(!cfg.is_security_relevant());
    }

    #[test]
    fn filter_matches_category() {
        let filter = AuditFilter {
            category: Some(AuditCategory::Security),
            actor: None,
            success_only: false,
        };
        let sec = make_event(1, AuditCategory::Security, "a", "op", true);
        let ua = make_event(2, AuditCategory::UserAction, "a", "op", true);
        assert!(filter.matches(&sec));
        assert!(!filter.matches(&ua));
    }

    #[test]
    fn filter_success_only() {
        let filter = AuditFilter {
            category: None,
            actor: None,
            success_only: true,
        };
        let ok = make_event(1, AuditCategory::SystemOp, "a", "op", true);
        let fail = make_event(2, AuditCategory::SystemOp, "a", "op", false);
        assert!(filter.matches(&ok));
        assert!(!filter.matches(&fail));
    }

    #[test]
    fn log_security_events_count() {
        let mut log = AuditLog::new();
        log.record(make_event(1, AuditCategory::Security, "a", "op", true));
        log.record(make_event(2, AuditCategory::DataAccess, "b", "read", true));
        log.record(make_event(3, AuditCategory::UserAction, "c", "click", true));
        let sec = log.security_events();
        assert_eq!(sec.len(), 2);
    }

    #[test]
    fn reporter_report_summary_format() {
        let mut r = AuditReporter::new();
        r.add(make_event(1, AuditCategory::Security, "a", "op", true));
        r.add(make_event(2, AuditCategory::UserAction, "b", "click", true));
        assert_eq!(r.report_summary(), "total:2 security:1");
    }
}
