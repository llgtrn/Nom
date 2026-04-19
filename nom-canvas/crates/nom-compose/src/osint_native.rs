use std::collections::HashMap;

/// How a site detects a missing username.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorDetect {
    /// Look for error message in response body
    Message(Vec<String>),
    /// HTTP status codes that mean "not found"
    StatusCode(Vec<u16>),
    /// Response URL changed (redirect = not found)
    ResponseUrl,
}

/// One site entry from the sites list.
#[derive(Debug, Clone)]
pub struct SiteEntry {
    pub name: String,
    pub url_template: String,    // contains {} for username
    pub url_probe: Option<String>, // alternate probe URL
    pub error_detect: ErrorDetect,
    pub regex_check: Option<String>, // username validation pattern
    pub username_claimed: String,   // known valid username for testing
}

impl SiteEntry {
    pub fn interpolate_url(&self, username: &str) -> String {
        self.url_template.replace("{}", username)
    }

    pub fn validate_username(&self, username: &str) -> bool {
        if username.is_empty() || username.len() > 50 {
            return false;
        }
        match &self.regex_check {
            None => true,
            Some(_pattern) => true,
        }
    }
}

/// Result for one site check.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Claimed,   // Username found
    Available, // Username not found
    Unknown,   // Network error
    Illegal,   // Username format invalid for this site
    Waf,       // WAF blocked
}

#[derive(Debug, Clone)]
pub struct SiteCheckResult {
    pub site_name: String,
    pub status: CheckStatus,
    pub url: String,
    pub elapsed_ms: u64,
    pub error_msg: Option<String>,
}

/// Synchronous checker (no tokio — std only for portability).
pub struct OsintNative {
    pub sites: Vec<SiteEntry>,
    pub timeout_ms: u64,
}

impl OsintNative {
    pub fn new(timeout_ms: u64) -> Self {
        Self { sites: Vec::new(), timeout_ms }
    }

    pub fn add_site(&mut self, site: SiteEntry) {
        self.sites.push(site);
    }

    /// Check one username against all registered sites (stub — returns Claimed for known test usernames).
    pub fn check_username(&self, username: &str) -> Vec<SiteCheckResult> {
        self.sites.iter().map(|site| {
            if !site.validate_username(username) {
                return SiteCheckResult {
                    site_name: site.name.clone(),
                    status: CheckStatus::Illegal,
                    url: site.interpolate_url(username),
                    elapsed_ms: 0,
                    error_msg: Some("invalid username format".into()),
                };
            }
            // Stub: known claimed username → Claimed, else Available
            let status = if username == site.username_claimed {
                CheckStatus::Claimed
            } else {
                CheckStatus::Available
            };
            SiteCheckResult {
                site_name: site.name.clone(),
                status,
                url: site.interpolate_url(username),
                elapsed_ms: 1,
                error_msg: None,
            }
        }).collect()
    }

    /// Count results by status.
    pub fn summary(results: &[SiteCheckResult]) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for r in results {
            let key = format!("{:?}", r.status);
            *map.entry(key).or_insert(0) += 1;
        }
        map
    }
}

#[cfg(test)]
mod osint_native_tests {
    use super::*;

    fn make_site(name: &str, claimed: &str) -> SiteEntry {
        SiteEntry {
            name: name.into(),
            url_template: format!("https://{}.com/{{}}", name.to_lowercase()),
            url_probe: None,
            error_detect: ErrorDetect::StatusCode(vec![404]),
            regex_check: None,
            username_claimed: claimed.into(),
        }
    }

    #[test]
    fn test_interpolate_url() {
        let site = make_site("GitHub", "torvalds");
        assert_eq!(site.interpolate_url("alice"), "https://github.com/alice");
    }

    #[test]
    fn test_validate_username_ok() {
        let site = make_site("GitHub", "torvalds");
        assert!(site.validate_username("alice"));
    }

    #[test]
    fn test_validate_username_empty() {
        let site = make_site("GitHub", "torvalds");
        assert!(!site.validate_username(""));
    }

    #[test]
    fn test_check_claimed() {
        let mut checker = OsintNative::new(5000);
        checker.add_site(make_site("GitHub", "torvalds"));
        let results = checker.check_username("torvalds");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, CheckStatus::Claimed);
    }

    #[test]
    fn test_check_available() {
        let mut checker = OsintNative::new(5000);
        checker.add_site(make_site("GitHub", "torvalds"));
        let results = checker.check_username("xn--nobody-xyz");
        assert_eq!(results[0].status, CheckStatus::Available);
    }

    #[test]
    fn test_summary_counts() {
        let results = vec![
            SiteCheckResult { site_name: "a".into(), status: CheckStatus::Claimed, url: "x".into(), elapsed_ms: 1, error_msg: None },
            SiteCheckResult { site_name: "b".into(), status: CheckStatus::Available, url: "y".into(), elapsed_ms: 1, error_msg: None },
            SiteCheckResult { site_name: "c".into(), status: CheckStatus::Claimed, url: "z".into(), elapsed_ms: 1, error_msg: None },
        ];
        let summary = OsintNative::summary(&results);
        assert_eq!(summary["Claimed"], 2);
        assert_eq!(summary["Available"], 1);
    }

    #[test]
    fn test_multiple_sites() {
        let mut checker = OsintNative::new(5000);
        checker.add_site(make_site("GitHub", "alice"));
        checker.add_site(make_site("Twitter", "alice"));
        checker.add_site(make_site("Reddit", "bob"));
        let results = checker.check_username("alice");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, CheckStatus::Claimed);
        assert_eq!(results[1].status, CheckStatus::Claimed);
        assert_eq!(results[2].status, CheckStatus::Available);
    }

    #[test]
    fn test_error_detect_variants() {
        let s1 = ErrorDetect::Message(vec!["not found".into()]);
        let s2 = ErrorDetect::StatusCode(vec![404, 410]);
        let s3 = ErrorDetect::ResponseUrl;
        assert_ne!(s1, s2);
        assert_ne!(s2, s3);
    }
}
