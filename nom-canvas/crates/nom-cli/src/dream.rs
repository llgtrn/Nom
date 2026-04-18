pub struct DreamConfig {
    pub target_score: f32,
    pub max_iterations: u32,
}

impl DreamConfig {
    pub fn new(target_score: f32, max_iterations: u32) -> Self {
        Self { target_score, max_iterations }
    }
}

impl Default for DreamConfig {
    fn default() -> Self {
        Self::new(95.0, 10)
    }
}

pub struct DreamIteration {
    pub iteration: u32,
    pub score: f32,
    pub proposals: Vec<String>,
}

impl DreamIteration {
    pub fn new(iteration: u32, score: f32, proposals: Vec<String>) -> Self {
        Self { iteration, score, proposals }
    }

    pub fn is_epic(&self) -> bool {
        self.score >= 95.0
    }
}

pub struct DreamEngine {
    pub config: DreamConfig,
}

impl DreamEngine {
    pub fn new(config: DreamConfig) -> Self {
        Self { config }
    }

    pub fn run_iteration(&self, iteration: u32, base_score: f32) -> DreamIteration {
        let score = (base_score + (iteration as f32 * 5.0)).min(100.0);
        DreamIteration::new(iteration, score, vec!["improve X".to_string(), "refine Y".to_string()])
    }

    pub fn run_until_epic(&self) -> Vec<DreamIteration> {
        let mut history = Vec::new();
        let mut base_score = 0.0f32;
        for i in 1..=self.config.max_iterations {
            let iter = self.run_iteration(i, base_score);
            let epic = iter.is_epic();
            history.push(iter);
            if epic {
                break;
            }
            base_score = history.last().unwrap().score;
        }
        history
    }

    pub fn final_score(iterations: &[DreamIteration]) -> f32 {
        iterations.last().map(|i| i.score).unwrap_or(0.0)
    }
}

pub struct DreamReport {
    pub iterations: Vec<DreamIteration>,
    pub final_score: f32,
    pub reached_epic: bool,
}

impl DreamReport {
    pub fn from_engine(engine: &DreamEngine) -> Self {
        let iterations = engine.run_until_epic();
        let final_score = DreamEngine::final_score(&iterations);
        let reached_epic = iterations.last().map(|i| i.is_epic()).unwrap_or(false);
        Self { iterations, final_score, reached_epic }
    }
}

#[cfg(test)]
mod dream_tests {
    use super::*;

    #[test]
    fn test_dream_config_defaults() {
        let cfg = DreamConfig::default();
        assert_eq!(cfg.target_score, 95.0);
        assert_eq!(cfg.max_iterations, 10);
    }

    #[test]
    fn test_dream_iteration_is_epic_at_95() {
        let iter = DreamIteration::new(1, 95.0, vec![]);
        assert!(iter.is_epic());
        let iter2 = DreamIteration::new(1, 94.9, vec![]);
        assert!(!iter2.is_epic());
    }

    #[test]
    fn test_run_iteration_returns_correct_iteration_number() {
        let engine = DreamEngine::new(DreamConfig::default());
        let iter = engine.run_iteration(3, 0.0);
        assert_eq!(iter.iteration, 3);
    }

    #[test]
    fn test_run_until_epic_terminates_at_epic_score() {
        // base_score=80 + iter*5: iter1=85, iter2=90, iter3=95 (epic)
        let engine = DreamEngine::new(DreamConfig::new(95.0, 10));
        // Manually set up: run_iteration uses cumulative base, so we need
        // a base_score that reaches epic quickly.
        // With base=0: iter1=5, iter2=10... never reaches 95 in 10 iters (max=50)
        // Use a config with high enough max to reach, or test with custom base.
        // The run_until_epic accumulates: iter1 base=0 score=5, iter2 base=5 score=15...
        // Let's just verify it terminates at is_epic
        let history = engine.run_until_epic();
        // With default config max_iterations=10, base starts at 0
        // scores: 5, 15, 30, 50, 75 (iter5: base=50+5*5=75... wait let me trace:
        // i=1: score=(0+1*5).min(100)=5; i=2: base=5, score=(5+2*5)=15;
        // i=3: base=15, score=(15+3*5)=30; i=4: base=30, score=(30+4*5)=50;
        // i=5: base=50, score=(50+5*5)=75; i=6: base=75, score=(75+6*5)=105.min(100)=100
        // i=6 score=100 >= 95, so epic at iteration 6
        assert!(history.last().unwrap().is_epic());
    }

    #[test]
    fn test_run_until_epic_terminates_at_max_iterations_if_never_epic() {
        // Use max_iterations=1 with a base that won't reach epic
        let engine = DreamEngine::new(DreamConfig::new(95.0, 1));
        // iter1: score=(0+1*5)=5, not epic, max_iterations=1 reached
        let history = engine.run_until_epic();
        assert_eq!(history.len(), 1);
        assert!(!history.last().unwrap().is_epic());
    }

    #[test]
    fn test_final_score_returns_last_score() {
        let iters = vec![
            DreamIteration::new(1, 30.0, vec![]),
            DreamIteration::new(2, 60.0, vec![]),
            DreamIteration::new(3, 90.0, vec![]),
        ];
        assert_eq!(DreamEngine::final_score(&iters), 90.0);
    }

    #[test]
    fn test_final_score_empty_returns_zero() {
        assert_eq!(DreamEngine::final_score(&[]), 0.0);
    }

    #[test]
    fn test_dream_report_reached_epic_correct() {
        let engine = DreamEngine::new(DreamConfig::default());
        let report = DreamReport::from_engine(&engine);
        // With default config (max=10), we traced above that iter 6 reaches 100
        assert!(report.reached_epic);
        assert!(report.final_score >= 95.0);
    }

    #[test]
    fn test_run_until_epic_history_length_le_max_iterations() {
        let engine = DreamEngine::new(DreamConfig::default());
        let history = engine.run_until_epic();
        assert!(history.len() <= engine.config.max_iterations as usize);
    }
}
