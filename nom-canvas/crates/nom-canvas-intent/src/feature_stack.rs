/// Intern table mapping string words to unique u32 IDs.
#[derive(Debug, Default, Clone)]
pub struct WordIdMap {
    words: Vec<String>,
}

impl WordIdMap {
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }

    /// Returns the existing ID for `word`, or assigns the next available ID.
    pub fn intern(&mut self, word: impl Into<String>) -> u32 {
        let word = word.into();
        if let Some(pos) = self.words.iter().position(|w| w == &word) {
            return pos as u32;
        }
        let id = self.words.len() as u32;
        self.words.push(word);
        id
    }

    /// Looks up the word for a given ID.
    pub fn lookup(&self, id: u32) -> Option<&str> {
        self.words.get(id as usize).map(|s| s.as_str())
    }

    /// Returns the number of interned words.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// A weighted feature identified by a word ID.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureWeight {
    pub word_id: u32,
    pub weight: f32,
}

impl FeatureWeight {
    pub fn new(word_id: u32, weight: f32) -> Self {
        Self { word_id, weight }
    }

    /// Returns true when this feature dominates (weight > 0.5).
    pub fn is_dominant(&self) -> bool {
        self.weight > 0.5
    }
}

/// Ordered stack of weighted features for a concept (MECE objective decomposition).
#[derive(Debug, Default, Clone)]
pub struct FeatureStack {
    features: Vec<FeatureWeight>,
}

impl FeatureStack {
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
        }
    }

    pub fn push(&mut self, feature: FeatureWeight) {
        self.features.push(feature);
    }

    /// Sum of all feature weights.
    pub fn total_weight(&self) -> f32 {
        self.features.iter().map(|f| f.weight).sum()
    }

    /// All features with `is_dominant() == true`.
    pub fn dominant_features(&self) -> Vec<&FeatureWeight> {
        self.features.iter().filter(|f| f.is_dominant()).collect()
    }

    /// Divides each weight by `total_weight`. No-op when total is zero.
    pub fn normalize(&mut self) {
        let total = self.total_weight();
        if total == 0.0 {
            return;
        }
        for f in &mut self.features {
            f.weight /= total;
        }
    }

    /// Returns references to the top-`k` features by weight (highest first).
    pub fn top_k(&self, k: usize) -> Vec<&FeatureWeight> {
        let mut indexed: Vec<&FeatureWeight> = self.features.iter().collect();
        indexed.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        indexed.into_iter().take(k).collect()
    }
}

#[cfg(test)]
mod feature_stack_tests {
    use super::*;

    #[test]
    fn word_id_map_intern_new() {
        let mut map = WordIdMap::new();
        let id = map.intern("alpha");
        assert_eq!(id, 0);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn word_id_map_intern_returns_same_id() {
        let mut map = WordIdMap::new();
        let id1 = map.intern("alpha");
        let id2 = map.intern("alpha");
        assert_eq!(id1, id2);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn word_id_map_lookup() {
        let mut map = WordIdMap::new();
        let id = map.intern("beta");
        assert_eq!(map.lookup(id), Some("beta"));
        assert_eq!(map.lookup(99), None);
    }

    #[test]
    fn feature_weight_is_dominant_true() {
        let fw = FeatureWeight::new(0, 0.8);
        assert!(fw.is_dominant());
    }

    #[test]
    fn feature_weight_is_dominant_false() {
        let fw = FeatureWeight::new(0, 0.3);
        assert!(!fw.is_dominant());
    }

    #[test]
    fn feature_stack_total_weight() {
        let mut stack = FeatureStack::new();
        stack.push(FeatureWeight::new(0, 0.4));
        stack.push(FeatureWeight::new(1, 0.6));
        let total = stack.total_weight();
        assert!((total - 1.0).abs() < 1e-6, "expected 1.0, got {total}");
    }

    #[test]
    fn feature_stack_dominant_features() {
        let mut stack = FeatureStack::new();
        stack.push(FeatureWeight::new(0, 0.3));
        stack.push(FeatureWeight::new(1, 0.7));
        stack.push(FeatureWeight::new(2, 0.9));
        let dominant = stack.dominant_features();
        assert_eq!(dominant.len(), 2);
        assert!(dominant.iter().all(|f| f.is_dominant()));
    }

    #[test]
    fn feature_stack_normalize() {
        let mut stack = FeatureStack::new();
        stack.push(FeatureWeight::new(0, 1.0));
        stack.push(FeatureWeight::new(1, 3.0));
        stack.normalize();
        let total = stack.total_weight();
        assert!((total - 1.0).abs() < 1e-6, "after normalize total should be 1.0, got {total}");
        assert!((stack.features[0].weight - 0.25).abs() < 1e-6);
        assert!((stack.features[1].weight - 0.75).abs() < 1e-6);
    }

    #[test]
    fn feature_stack_top_k() {
        let mut stack = FeatureStack::new();
        stack.push(FeatureWeight::new(0, 0.1));
        stack.push(FeatureWeight::new(1, 0.9));
        stack.push(FeatureWeight::new(2, 0.5));
        let top2 = stack.top_k(2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].word_id, 1); // 0.9
        assert_eq!(top2[1].word_id, 2); // 0.5
    }
}
