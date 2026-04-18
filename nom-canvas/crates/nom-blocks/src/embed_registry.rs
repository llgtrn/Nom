/// EmbedKind — the type of embedded content.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbedKind {
    /// Raster or vector image.
    Image,
    /// Video stream.
    Video,
    /// Audio stream.
    Audio,
    /// Generic document (PDF, DOCX, etc.).
    Document,
    /// Source code snippet.
    Code,
}

impl EmbedKind {
    /// Returns true for media kinds (Image, Video, Audio).
    pub fn is_media(&self) -> bool {
        matches!(self, EmbedKind::Image | EmbedKind::Video | EmbedKind::Audio)
    }

    /// Returns the MIME type prefix for this kind.
    pub fn mime_prefix(&self) -> &'static str {
        match self {
            EmbedKind::Image => "image",
            EmbedKind::Video => "video",
            EmbedKind::Audio => "audio",
            EmbedKind::Document => "application",
            EmbedKind::Code => "text",
        }
    }
}

/// A single embedded resource entry.
pub struct EmbedEntry {
    /// Unique identifier.
    pub id: u64,
    /// Kind of embedded content.
    pub kind: EmbedKind,
    /// Source URL or path.
    pub source_url: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Whether the embed has been resolved (fetched / validated).
    pub resolved: bool,
}

impl EmbedEntry {
    /// Returns true when the entry exceeds 5 MB.
    pub fn is_large(&self) -> bool {
        self.size_bytes > 5_000_000
    }

    /// Marks this entry as resolved.
    pub fn mark_resolved(&mut self) {
        self.resolved = true;
    }
}

/// In-memory registry of embed entries keyed by id.
pub struct EmbedRegistry {
    /// All registered entries.
    pub entries: std::collections::HashMap<u64, EmbedEntry>,
}

impl EmbedRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self { entries: std::collections::HashMap::new() }
    }

    /// Inserts an entry, replacing any existing entry with the same id.
    pub fn register(&mut self, entry: EmbedEntry) {
        self.entries.insert(entry.id, entry);
    }

    /// Returns a reference to the entry with the given id, if present.
    pub fn get(&self, id: u64) -> Option<&EmbedEntry> {
        self.entries.get(&id)
    }

    /// Returns all entries whose kind matches `k`.
    pub fn by_kind(&self, k: &EmbedKind) -> Vec<&EmbedEntry> {
        self.entries.values().filter(|e| &e.kind == k).collect()
    }

    /// Returns all entries that have not yet been resolved.
    pub fn unresolved(&self) -> Vec<&EmbedEntry> {
        self.entries.values().filter(|e| !e.resolved).collect()
    }
}

impl Default for EmbedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level resolver that wraps an `EmbedRegistry`.
pub struct EmbedResolver {
    registry: EmbedRegistry,
}

impl EmbedResolver {
    /// Creates a new resolver with an empty registry.
    pub fn new() -> Self {
        Self { registry: EmbedRegistry::new() }
    }

    /// Delegates to `EmbedRegistry::register`.
    pub fn add(&mut self, e: EmbedEntry) {
        self.registry.register(e);
    }

    /// Marks the entry with the given id as resolved.
    /// Returns `true` if the entry was found, `false` otherwise.
    pub fn resolve(&mut self, id: u64) -> bool {
        match self.registry.entries.get_mut(&id) {
            Some(entry) => {
                entry.mark_resolved();
                true
            }
            None => false,
        }
    }

    /// Returns the count of resolved entries.
    pub fn resolved_count(&self) -> usize {
        self.registry.entries.values().filter(|e| e.resolved).count()
    }
}

impl Default for EmbedResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod embed_registry_tests {
    use super::*;

    fn make_entry(id: u64, kind: EmbedKind, size_bytes: u64, resolved: bool) -> EmbedEntry {
        EmbedEntry {
            id,
            kind,
            source_url: format!("https://example.com/{id}"),
            size_bytes,
            resolved,
        }
    }

    #[test]
    fn kind_is_media_image_true() {
        assert!(EmbedKind::Image.is_media());
    }

    #[test]
    fn kind_is_media_document_false() {
        assert!(!EmbedKind::Document.is_media());
    }

    #[test]
    fn kind_mime_prefix() {
        assert_eq!(EmbedKind::Image.mime_prefix(), "image");
        assert_eq!(EmbedKind::Video.mime_prefix(), "video");
        assert_eq!(EmbedKind::Audio.mime_prefix(), "audio");
        assert_eq!(EmbedKind::Document.mime_prefix(), "application");
        assert_eq!(EmbedKind::Code.mime_prefix(), "text");
    }

    #[test]
    fn entry_is_large() {
        let small = make_entry(1, EmbedKind::Image, 1_000_000, false);
        let large = make_entry(2, EmbedKind::Image, 6_000_000, false);
        assert!(!small.is_large());
        assert!(large.is_large());
    }

    #[test]
    fn entry_mark_resolved() {
        let mut entry = make_entry(3, EmbedKind::Audio, 100, false);
        assert!(!entry.resolved);
        entry.mark_resolved();
        assert!(entry.resolved);
    }

    #[test]
    fn registry_by_kind_count() {
        let mut reg = EmbedRegistry::new();
        reg.register(make_entry(1, EmbedKind::Image, 100, false));
        reg.register(make_entry(2, EmbedKind::Image, 200, false));
        reg.register(make_entry(3, EmbedKind::Video, 300, false));
        assert_eq!(reg.by_kind(&EmbedKind::Image).len(), 2);
        assert_eq!(reg.by_kind(&EmbedKind::Video).len(), 1);
        assert_eq!(reg.by_kind(&EmbedKind::Audio).len(), 0);
    }

    #[test]
    fn registry_unresolved_filter() {
        let mut reg = EmbedRegistry::new();
        reg.register(make_entry(1, EmbedKind::Image, 100, false));
        reg.register(make_entry(2, EmbedKind::Image, 200, true));
        reg.register(make_entry(3, EmbedKind::Video, 300, false));
        assert_eq!(reg.unresolved().len(), 2);
    }

    #[test]
    fn resolver_resolve_true() {
        let mut resolver = EmbedResolver::new();
        resolver.add(make_entry(10, EmbedKind::Document, 500, false));
        assert!(resolver.resolve(10));
        assert!(!resolver.resolve(99)); // not found
    }

    #[test]
    fn resolver_resolved_count() {
        let mut resolver = EmbedResolver::new();
        resolver.add(make_entry(1, EmbedKind::Code, 50, false));
        resolver.add(make_entry(2, EmbedKind::Code, 60, false));
        resolver.add(make_entry(3, EmbedKind::Code, 70, true));
        assert_eq!(resolver.resolved_count(), 1);
        resolver.resolve(1);
        assert_eq!(resolver.resolved_count(), 2);
    }
}
