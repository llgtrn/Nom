#![deny(unsafe_code)]

/// Result from querying grammar.kinds
#[derive(Debug, Clone)]
pub struct KindResult {
    pub name: String,
    pub description: String,
    pub status: String,
}

/// Queries grammar.kinds via bridge
pub struct KindQueryClient {
    pub kind_count: u32,
    pub results: Vec<KindResult>,
}

impl KindQueryClient {
    pub fn new() -> Self {
        Self {
            kind_count: 0,
            results: Vec::new(),
        }
    }

    /// Populate with N mock kinds
    pub fn simulate_load(&mut self, count: u32) {
        self.kind_count = count;
        self.results = (0..count)
            .map(|i| KindResult {
                name: format!("kind_{i}"),
                description: format!("description for kind {i}"),
                status: if i % 2 == 0 {
                    "active".to_string()
                } else {
                    "draft".to_string()
                },
            })
            .collect();
    }

    pub fn find_by_name(&self, name: &str) -> Option<&KindResult> {
        self.results.iter().find(|r| r.name == name)
    }

    /// Returns all results with status = "active"
    pub fn list_active(&self) -> Vec<&KindResult> {
        self.results.iter().filter(|r| r.status == "active").collect()
    }

    pub fn count(&self) -> usize {
        self.results.len()
    }
}

impl Default for KindQueryClient {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL wrapper stub — lists all kind names from the client
pub fn list_kinds_stub(client: &KindQueryClient) -> Vec<String> {
    client.results.iter().map(|r| r.name.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_query_client_new_empty() {
        let client = KindQueryClient::new();
        assert_eq!(client.kind_count, 0);
        assert!(client.results.is_empty());
        assert_eq!(client.count(), 0);
    }

    #[test]
    fn kind_query_simulate_load() {
        let mut client = KindQueryClient::new();
        client.simulate_load(5);
        assert_eq!(client.kind_count, 5);
        assert_eq!(client.count(), 5);
    }

    #[test]
    fn kind_query_find_by_name() {
        let mut client = KindQueryClient::new();
        client.simulate_load(3);
        let result = client.find_by_name("kind_1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "kind_1");
    }

    #[test]
    fn kind_query_list_active() {
        let mut client = KindQueryClient::new();
        client.simulate_load(4);
        // Even indices (0, 2) are "active"; odd (1, 3) are "draft"
        let active = client.list_active();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|r| r.status == "active"));
    }

    #[test]
    fn list_kinds_stub_returns_names() {
        let mut client = KindQueryClient::new();
        client.simulate_load(3);
        let names = list_kinds_stub(&client);
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "kind_0");
        assert_eq!(names[1], "kind_1");
        assert_eq!(names[2], "kind_2");
    }
}
