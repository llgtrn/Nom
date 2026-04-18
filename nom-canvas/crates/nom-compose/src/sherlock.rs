//! Sherlock OSINT adapter — wraps the Sherlock username-search CLI.
//!
//! Sherlock CLI reference (`python3 -m sherlock_project <username>`):
//!   --json        emit one JSON object per line with site results
//!
//! The JSON format we parse (batch export):
//!   {"username": "...", "sites": [{"name": "...", "url": "...", "status": "found"}, ...]}
//!
//! `status` values from Sherlock source (QueryStatus enum):
//!   "claimed" / "found"       → SherlockStatus::Found
//!   "available" / "not found" → SherlockStatus::NotFound
//!   "unknown"                 → SherlockStatus::Unknown
//!   everything else           → SherlockStatus::Error

use crate::inspector::InspectFinding;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Status of a single site lookup, mirroring Sherlock's QueryStatus enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SherlockStatus {
    /// Account found / username claimed on this site.
    Found,
    /// Username not present on this site.
    NotFound,
    /// Sherlock could not determine presence.
    Unknown,
    /// Network/HTTP error during lookup.
    Error,
}

impl SherlockStatus {
    /// Parse a Sherlock status string (case-insensitive).
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().trim() {
            "claimed" | "found" => Self::Found,
            "available" | "not found" | "not_found" => Self::NotFound,
            "unknown" => Self::Unknown,
            _ => Self::Error,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::NotFound => "not_found",
            Self::Unknown => "unknown",
            Self::Error => "error",
        }
    }
}

/// One site result from a Sherlock run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SherlockSite {
    pub name: String,
    pub url: String,
    pub status: SherlockStatus,
}

impl SherlockSite {
    /// Construct a site record.
    pub fn new(name: impl Into<String>, url: impl Into<String>, status: SherlockStatus) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            status,
        }
    }
}

/// Aggregated result of a Sherlock username search.
#[derive(Debug, Clone)]
pub struct SherlockResult {
    pub username: String,
    pub sites: Vec<SherlockSite>,
    pub found_count: usize,
    pub elapsed_ms: u64,
}

impl SherlockResult {
    /// Create an empty result for the given username.
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            sites: Vec::new(),
            found_count: 0,
            elapsed_ms: 0,
        }
    }

    /// Add a site record and keep `found_count` in sync.
    pub fn add_site(&mut self, site: SherlockSite) {
        if site.status == SherlockStatus::Found {
            self.found_count += 1;
        }
        self.sites.push(site);
    }

    /// Return references to every site where the username was found.
    pub fn found_sites(&self) -> Vec<&SherlockSite> {
        self.sites
            .iter()
            .filter(|s| s.status == SherlockStatus::Found)
            .collect()
    }

    /// Emit a `.nomx` representation of the person's online presence.
    ///
    /// Format: `define person that username(<name>) profiles(<site1>,<site2>,...)`
    pub fn to_nomx(&self) -> String {
        let profiles: Vec<&str> = self
            .found_sites()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        format!(
            "define person that username({}) profiles({})",
            self.username,
            profiles.join(",")
        )
    }
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Sherlock OSINT adapter.
pub struct SherlockAdapter;

impl SherlockAdapter {
    /// Create a new adapter instance.
    pub fn new() -> Self {
        Self
    }

    /// Return a stub result containing well-known sites so tests can run
    /// without invoking the real Sherlock CLI.
    ///
    /// Three sites are marked `Found`, one as `NotFound`.
    pub fn run_stub(username: &str) -> SherlockResult {
        let mut result = SherlockResult::new(username);
        result.add_site(SherlockSite::new(
            "github",
            format!("https://github.com/{username}"),
            SherlockStatus::Found,
        ));
        result.add_site(SherlockSite::new(
            "linkedin",
            format!("https://www.linkedin.com/in/{username}"),
            SherlockStatus::Found,
        ));
        result.add_site(SherlockSite::new(
            "twitter",
            format!("https://twitter.com/{username}"),
            SherlockStatus::Found,
        ));
        result.add_site(SherlockSite::new(
            "instagram",
            format!("https://www.instagram.com/{username}"),
            SherlockStatus::NotFound,
        ));
        result
    }

    /// Parse Sherlock's batch JSON output.
    ///
    /// Expected format:
    /// ```json
    /// {"username": "alice", "sites": [{"name": "GitHub", "url": "https://github.com/alice", "status": "found"}]}
    /// ```
    pub fn parse_json_output(json: &str) -> SherlockResult {
        let username = extract_json_string(json, "username").unwrap_or_default();
        let mut result = SherlockResult::new(username);

        if let Some(sites_start) = json.find("\"sites\"") {
            let after_key = &json[sites_start + 7..];
            if let Some(arr_start) = after_key.find('[') {
                let arr_body = &after_key[arr_start + 1..];
                let mut remaining = arr_body;
                while let Some(obj_start) = remaining.find('{') {
                    let obj_body = &remaining[obj_start + 1..];
                    if let Some(obj_end) = obj_body.find('}') {
                        let obj = &obj_body[..obj_end];
                        let name = extract_json_string(obj, "name").unwrap_or_default();
                        let url = extract_json_string(obj, "url").unwrap_or_default();
                        let status_str = extract_json_string(obj, "status").unwrap_or_default();
                        let status = SherlockStatus::parse(&status_str);
                        result.add_site(SherlockSite::new(name, url, status));
                        remaining = &obj_body[obj_end + 1..];
                    } else {
                        break;
                    }
                }
            }
        }
        result
    }

    /// Convert each `Found` site in a `SherlockResult` to an `InspectFinding`.
    ///
    /// Category: `"profile"`, key: `"platform"`, value: site name, confidence: 1.0.
    pub fn to_inspect_findings(result: &SherlockResult) -> Vec<InspectFinding> {
        result
            .found_sites()
            .into_iter()
            .map(|site| InspectFinding::new("profile", "platform", &site.name, 1.0))
            .collect()
    }
}

impl Default for SherlockAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

/// Extract the string value for a JSON key from a flat object body.
fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let search = format!("\"{key}\"");
    let pos = text.find(&search)?;
    let after_key = &text[pos + search.len()..];
    let after_colon = after_key.trim_start_matches([' ', '\t', '\n', '\r', ':']);
    if let Some(inner) = after_colon.strip_prefix('"') {
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. site_new
    #[test]
    fn site_new() {
        let site = SherlockSite::new("github", "https://github.com/user", SherlockStatus::Found);
        assert_eq!(site.name, "github");
        assert_eq!(site.url, "https://github.com/user");
        assert_eq!(site.status, SherlockStatus::Found);
    }

    // 2. status_variants
    #[test]
    fn status_variants() {
        assert_eq!(SherlockStatus::parse("claimed"), SherlockStatus::Found);
        assert_eq!(SherlockStatus::parse("found"), SherlockStatus::Found);
        assert_eq!(SherlockStatus::parse("available"), SherlockStatus::NotFound);
        assert_eq!(SherlockStatus::parse("not found"), SherlockStatus::NotFound);
        assert_eq!(SherlockStatus::parse("unknown"), SherlockStatus::Unknown);
        assert_eq!(SherlockStatus::parse("timeout"), SherlockStatus::Error);
        assert_eq!(SherlockStatus::Found.label(), "found");
        assert_eq!(SherlockStatus::NotFound.label(), "not_found");
    }

    // 3. result_add_site
    #[test]
    fn result_add_site() {
        let mut r = SherlockResult::new("alice");
        assert_eq!(r.sites.len(), 0);
        r.add_site(SherlockSite::new(
            "github",
            "https://github.com/alice",
            SherlockStatus::Found,
        ));
        assert_eq!(r.sites.len(), 1);
        assert_eq!(r.found_count, 1);
        r.add_site(SherlockSite::new(
            "fb",
            "https://fb.com/alice",
            SherlockStatus::NotFound,
        ));
        assert_eq!(r.sites.len(), 2);
        assert_eq!(r.found_count, 1, "NotFound must not increment found_count");
    }

    // 4. result_found_count
    #[test]
    fn result_found_count() {
        let mut r = SherlockResult::new("bob");
        r.add_site(SherlockSite::new("a", "http://a.com", SherlockStatus::Found));
        r.add_site(SherlockSite::new("b", "http://b.com", SherlockStatus::Found));
        r.add_site(SherlockSite::new("c", "http://c.com", SherlockStatus::NotFound));
        r.add_site(SherlockSite::new("d", "http://d.com", SherlockStatus::Error));
        assert_eq!(r.found_count, 2);
        assert_eq!(r.found_sites().len(), 2);
    }

    // 5. result_to_nomx
    #[test]
    fn result_to_nomx() {
        let mut r = SherlockResult::new("charlie");
        r.add_site(SherlockSite::new(
            "github",
            "https://github.com/charlie",
            SherlockStatus::Found,
        ));
        r.add_site(SherlockSite::new(
            "linkedin",
            "https://linkedin.com/in/charlie",
            SherlockStatus::Found,
        ));
        r.add_site(SherlockSite::new(
            "fb",
            "https://fb.com/charlie",
            SherlockStatus::NotFound,
        ));
        let nomx = r.to_nomx();
        assert!(
            nomx.contains("define person"),
            "must start with define person, got: {nomx}"
        );
        assert!(
            nomx.contains("username(charlie)"),
            "must include username, got: {nomx}"
        );
        assert!(
            nomx.contains("github"),
            "must include found site github, got: {nomx}"
        );
        assert!(
            nomx.contains("linkedin"),
            "must include found site linkedin, got: {nomx}"
        );
        assert!(
            !nomx.contains("fb"),
            "must exclude NotFound site fb, got: {nomx}"
        );
    }

    // 6. adapter_run_stub_found
    #[test]
    fn adapter_run_stub_found() {
        let result = SherlockAdapter::run_stub("devuser");
        assert_eq!(result.username, "devuser");
        assert_eq!(result.found_count, 3, "stub must have 3 Found sites");
        assert_eq!(result.sites.len(), 4, "stub must have 4 total sites");
        let names: Vec<&str> = result
            .found_sites()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"github"), "github must be Found");
        assert!(names.contains(&"linkedin"), "linkedin must be Found");
        assert!(names.contains(&"twitter"), "twitter must be Found");
        let not_found: Vec<&SherlockSite> = result
            .sites
            .iter()
            .filter(|s| s.status == SherlockStatus::NotFound)
            .collect();
        assert_eq!(not_found.len(), 1);
        assert_eq!(not_found[0].name, "instagram");
    }

    // 7. adapter_parse_json
    #[test]
    fn adapter_parse_json() {
        let json = r#"{"username": "diana", "sites": [{"name": "GitHub", "url": "https://github.com/diana", "status": "found"}, {"name": "Reddit", "url": "https://reddit.com/u/diana", "status": "not found"}, {"name": "HackerNews", "url": "https://news.ycombinator.com/user?id=diana", "status": "claimed"}]}"#;
        let result = SherlockAdapter::parse_json_output(json);
        assert_eq!(result.username, "diana");
        assert_eq!(result.sites.len(), 3);
        assert_eq!(result.found_count, 2, "found + claimed both map to Found");
        let found_names: Vec<&str> = result
            .found_sites()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(found_names.contains(&"GitHub"));
        assert!(found_names.contains(&"HackerNews"));
    }

    // 8. adapter_to_findings
    #[test]
    fn adapter_to_findings() {
        let result = SherlockAdapter::run_stub("eve");
        let findings = SherlockAdapter::to_inspect_findings(&result);
        assert_eq!(findings.len(), 3, "one finding per Found site");
        for f in &findings {
            assert_eq!(f.category, "profile");
            assert_eq!(f.key, "platform");
            assert!(!f.value.is_empty());
            assert!((f.confidence - 1.0).abs() < f32::EPSILON);
        }
        let values: Vec<&str> = findings.iter().map(|f| f.value.as_str()).collect();
        assert!(values.contains(&"github"));
        assert!(values.contains(&"linkedin"));
        assert!(values.contains(&"twitter"));
    }
}
