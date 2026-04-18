#![deny(unsafe_code)]

// ---------------------------------------------------------------------------
// InspectKind — the category of entity being inspected
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectKind {
    YoutubeChannel,
    GithubRepo,
    Website,
    Person,
    Company,
    Unknown,
}

// ---------------------------------------------------------------------------
// InspectRequest — a parsed inspection target
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InspectRequest {
    pub input: String,
    pub kind: InspectKind,
}

impl InspectRequest {
    /// Detect kind from input:
    /// - "youtube.com" or "youtu.be" → YoutubeChannel
    /// - "github.com" → GithubRepo
    /// - starts with "http" → Website
    /// - no dots / no spaces (single word) → Person
    /// - has dot but not http → Company
    /// - else → Unknown
    pub fn new(input: &str) -> Self {
        let lc = input.to_lowercase();
        let kind = if lc.contains("youtube.com") || lc.contains("youtu.be") {
            InspectKind::YoutubeChannel
        } else if lc.contains("github.com") {
            InspectKind::GithubRepo
        } else if lc.starts_with("http") {
            InspectKind::Website
        } else if !lc.contains('.') && !lc.contains(' ') {
            InspectKind::Person
        } else if lc.contains('.') {
            InspectKind::Company
        } else {
            InspectKind::Unknown
        };

        Self {
            input: input.to_string(),
            kind,
        }
    }

    /// Returns `true` when the input starts with "http".
    pub fn is_url(&self) -> bool {
        self.input.to_lowercase().starts_with("http")
    }

    /// Human-readable label for the detected kind.
    pub fn kind_label(&self) -> &str {
        match self.kind {
            InspectKind::YoutubeChannel => "youtube-channel",
            InspectKind::GithubRepo => "github-repo",
            InspectKind::Website => "website",
            InspectKind::Person => "person",
            InspectKind::Company => "company",
            InspectKind::Unknown => "unknown",
        }
    }
}

// ---------------------------------------------------------------------------
// InspectResult — the outcome of an inspection run
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InspectResult {
    pub request: InspectRequest,
    pub findings_count: usize,
    pub nomx_preview: String,
    pub canvas_mode: String,
}

impl InspectResult {
    pub fn new(request: InspectRequest, findings_count: usize, nomx_preview: &str) -> Self {
        let canvas_mode = match request.kind {
            InspectKind::YoutubeChannel => "compose",
            InspectKind::GithubRepo => "canvas",
            InspectKind::Person | InspectKind::Company => "document",
            _ => "editor",
        }
        .to_string();

        Self {
            request,
            findings_count,
            nomx_preview: nomx_preview.to_string(),
            canvas_mode,
        }
    }
}

// ---------------------------------------------------------------------------
// InspectPanel — stateful inspector with history
// ---------------------------------------------------------------------------

pub struct InspectPanel {
    pub history: Vec<InspectResult>,
}

impl InspectPanel {
    pub fn new() -> Self {
        Self { history: vec![] }
    }

    /// Parse `input`, run a stub inspection, record the result, and return it.
    pub fn inspect(&mut self, input: &str) -> InspectResult {
        let request = InspectRequest::new(input);
        let label = request.kind_label().to_string();
        let nomx_preview = format!("entry {{ kind: \"{}\", source: \"{}\" }}", label, input);
        let result = InspectResult::new(request, 1, &nomx_preview);
        self.history.push(result.clone());
        result
    }

    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    pub fn last_result(&self) -> Option<&InspectResult> {
        self.history.last()
    }
}

impl Default for InspectPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_detect_youtube() {
        let r = InspectRequest::new("https://www.youtube.com/@mkbhd");
        assert_eq!(r.kind, InspectKind::YoutubeChannel);
        assert_eq!(r.kind_label(), "youtube-channel");

        let r2 = InspectRequest::new("https://youtu.be/dQw4w9WgXcQ");
        assert_eq!(r2.kind, InspectKind::YoutubeChannel);
    }

    #[test]
    fn request_detect_github() {
        let r = InspectRequest::new("https://github.com/rust-lang/rust");
        assert_eq!(r.kind, InspectKind::GithubRepo);
        assert_eq!(r.kind_label(), "github-repo");
    }

    #[test]
    fn request_detect_person() {
        let r = InspectRequest::new("elonmusk");
        assert_eq!(r.kind, InspectKind::Person);
        assert_eq!(r.kind_label(), "person");
    }

    #[test]
    fn request_detect_company() {
        let r = InspectRequest::new("anthropic.com");
        assert_eq!(r.kind, InspectKind::Company);
        assert_eq!(r.kind_label(), "company");
    }

    #[test]
    fn request_is_url() {
        let url = InspectRequest::new("https://example.com");
        assert!(url.is_url());

        let not_url = InspectRequest::new("someusername");
        assert!(!not_url.is_url());
    }

    #[test]
    fn result_canvas_mode_compose() {
        let req = InspectRequest::new("https://www.youtube.com/channel/UC_x5XG1OV2P6uZZ5FSM9Ttw");
        let result = InspectResult::new(req, 3, "entry { kind: \"youtube-channel\" }");
        assert_eq!(result.canvas_mode, "compose");
    }

    #[test]
    fn result_canvas_mode_document() {
        let req_person = InspectRequest::new("satyanadella");
        let result_person = InspectResult::new(req_person, 5, "entry { kind: \"person\" }");
        assert_eq!(result_person.canvas_mode, "document");

        let req_company = InspectRequest::new("microsoft.com");
        let result_company = InspectResult::new(req_company, 10, "entry { kind: \"company\" }");
        assert_eq!(result_company.canvas_mode, "document");
    }

    #[test]
    fn panel_inspect() {
        let mut panel = InspectPanel::new();
        let result = panel.inspect("https://github.com/tokio-rs/tokio");
        assert_eq!(result.request.kind, InspectKind::GithubRepo);
        assert_eq!(result.canvas_mode, "canvas");
        assert_eq!(result.findings_count, 1);
        assert!(result.nomx_preview.contains("github-repo"));
    }

    #[test]
    fn panel_history_count() {
        let mut panel = InspectPanel::new();
        assert_eq!(panel.history_count(), 0);
        panel.inspect("https://youtube.com/watch?v=abc");
        panel.inspect("tokiouser");
        assert_eq!(panel.history_count(), 2);
    }

    #[test]
    fn panel_last_result() {
        let mut panel = InspectPanel::new();
        assert!(panel.last_result().is_none());

        panel.inspect("https://github.com/zed-industries/zed");
        panel.inspect("anthropic.com");

        let last = panel.last_result().unwrap();
        assert_eq!(last.request.kind, InspectKind::Company);
        assert_eq!(last.canvas_mode, "document");
    }
}
