//! Retry + backoff helpers shared across compose backends.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackoffKind {
    /// Fixed wait between retries.
    Fixed,
    /// Exponential: wait = base * 2^(attempt - 1).
    Exponential,
    /// Exponential with ±25% jitter.
    ExponentialJitter,
    /// Full jitter: random in [0, base * 2^(attempt-1)].
    FullJitter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackoffConfig {
    pub kind: BackoffKind,
    pub base_ms: u32,
    pub max_ms: u32,
    pub max_attempts: u8,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self { kind: BackoffKind::ExponentialJitter, base_ms: 100, max_ms: 30_000, max_attempts: 5 }
    }
}

/// Deterministic wait calculator for a given `attempt` (1-based).
/// For `ExponentialJitter` + `FullJitter` requires a pseudorandom seed.
pub fn wait_ms(config: BackoffConfig, attempt: u8, rand_01: f32) -> u32 {
    if attempt == 0 { return 0; }
    let base = config.base_ms as u64;
    let max = config.max_ms as u64;
    let raw = match config.kind {
        BackoffKind::Fixed => base,
        BackoffKind::Exponential => base.saturating_mul(1u64 << (attempt - 1).min(31)),
        BackoffKind::ExponentialJitter => {
            let exp = base.saturating_mul(1u64 << (attempt - 1).min(31));
            let jitter = (rand_01.clamp(0.0, 1.0) - 0.5) * 0.5 * exp as f32; // ±25%
            (exp as f32 + jitter).max(0.0) as u64
        }
        BackoffKind::FullJitter => {
            let exp = base.saturating_mul(1u64 << (attempt - 1).min(31));
            (rand_01.clamp(0.0, 1.0) * exp as f32) as u64
        }
    };
    raw.min(max) as u32
}

pub fn should_retry(config: BackoffConfig, attempt: u8) -> bool {
    attempt < config.max_attempts
}

// ── Circuit breaker ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CircuitState { Closed, Open, HalfOpen }

#[derive(Clone, Copy, Debug)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub reset_after_ms: u64,
    pub half_open_success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self { failure_threshold: 5, reset_after_ms: 30_000, half_open_success_threshold: 2 }
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    opened_at_ms: Option<u64>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self { config, state: CircuitState::Closed, failure_count: 0, success_count: 0, opened_at_ms: None }
    }

    pub fn state(&self) -> CircuitState { self.state }

    pub fn record_success(&mut self, now_ms: u64) {
        match self.state {
            CircuitState::Closed => { self.failure_count = 0; }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.half_open_success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.opened_at_ms = None;
                }
            }
            CircuitState::Open => {
                let _ = now_ms;
            }
        }
    }

    pub fn record_failure(&mut self, now_ms: u64) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    self.state = CircuitState::Open;
                    self.opened_at_ms = Some(now_ms);
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.opened_at_ms = Some(now_ms);
                self.success_count = 0;
            }
            CircuitState::Open => { /* already open */ }
        }
    }

    /// Called before attempting a request. Transitions Open → HalfOpen when
    /// reset_after_ms has elapsed.
    pub fn poll(&mut self, now_ms: u64) -> CircuitState {
        if let CircuitState::Open = self.state {
            if let Some(opened) = self.opened_at_ms {
                if now_ms.saturating_sub(opened) >= self.config.reset_after_ms {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                }
            }
        }
        self.state
    }

    pub fn can_dispatch(&mut self, now_ms: u64) -> bool {
        !matches!(self.poll(now_ms), CircuitState::Open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BackoffConfig defaults ───────────────────────────────────────────────

    #[test]
    fn default_backoff_config_values() {
        let cfg = BackoffConfig::default();
        assert_eq!(cfg.kind, BackoffKind::ExponentialJitter);
        assert_eq!(cfg.base_ms, 100);
        assert_eq!(cfg.max_ms, 30_000);
        assert_eq!(cfg.max_attempts, 5);
    }

    // ── wait_ms ─────────────────────────────────────────────────────────────

    #[test]
    fn wait_ms_attempt_zero_returns_zero() {
        let cfg = BackoffConfig::default();
        assert_eq!(wait_ms(cfg, 0, 0.5), 0);
    }

    #[test]
    fn wait_ms_fixed_always_returns_base() {
        let cfg = BackoffConfig { kind: BackoffKind::Fixed, base_ms: 200, max_ms: 10_000, max_attempts: 5 };
        assert_eq!(wait_ms(cfg, 1, 0.0), 200);
        assert_eq!(wait_ms(cfg, 3, 0.9), 200);
        assert_eq!(wait_ms(cfg, 5, 0.5), 200);
    }

    #[test]
    fn wait_ms_exponential_doubles_each_attempt() {
        let cfg = BackoffConfig { kind: BackoffKind::Exponential, base_ms: 100, max_ms: 100_000, max_attempts: 5 };
        assert_eq!(wait_ms(cfg, 1, 0.5), 100);
        assert_eq!(wait_ms(cfg, 2, 0.5), 200);
        assert_eq!(wait_ms(cfg, 3, 0.5), 400);
        assert_eq!(wait_ms(cfg, 4, 0.5), 800);
    }

    #[test]
    fn wait_ms_respects_max_ms_cap() {
        let cfg = BackoffConfig { kind: BackoffKind::Exponential, base_ms: 1000, max_ms: 3_000, max_attempts: 10 };
        assert_eq!(wait_ms(cfg, 5, 0.5), 3_000);
        assert_eq!(wait_ms(cfg, 10, 0.5), 3_000);
    }

    #[test]
    fn wait_ms_exponential_jitter_with_rand_half_equals_exponential() {
        let cfg = BackoffConfig { kind: BackoffKind::ExponentialJitter, base_ms: 100, max_ms: 100_000, max_attempts: 5 };
        // rand=0.5 → jitter = (0.5-0.5)*0.5*exp = 0; result == exp
        assert_eq!(wait_ms(cfg, 1, 0.5), 100);
        assert_eq!(wait_ms(cfg, 2, 0.5), 200);
        assert_eq!(wait_ms(cfg, 3, 0.5), 400);
    }

    #[test]
    fn wait_ms_full_jitter_with_rand_zero_returns_zero() {
        let cfg = BackoffConfig { kind: BackoffKind::FullJitter, base_ms: 100, max_ms: 100_000, max_attempts: 5 };
        assert_eq!(wait_ms(cfg, 1, 0.0), 0);
        assert_eq!(wait_ms(cfg, 3, 0.0), 0);
    }

    #[test]
    fn wait_ms_full_jitter_with_rand_one_equals_exp() {
        let cfg = BackoffConfig { kind: BackoffKind::FullJitter, base_ms: 100, max_ms: 100_000, max_attempts: 5 };
        // rand=1.0 → exp * 1.0; attempt=1 → 100*1=100, attempt=2 → 200
        assert_eq!(wait_ms(cfg, 1, 1.0), 100);
        assert_eq!(wait_ms(cfg, 2, 1.0), 200);
    }

    // ── should_retry ─────────────────────────────────────────────────────────

    #[test]
    fn should_retry_true_when_below_max() {
        let cfg = BackoffConfig::default(); // max_attempts = 5
        assert!(should_retry(cfg, 0));
        assert!(should_retry(cfg, 4));
    }

    #[test]
    fn should_retry_false_when_at_or_above_max() {
        let cfg = BackoffConfig::default(); // max_attempts = 5
        assert!(!should_retry(cfg, 5));
        assert!(!should_retry(cfg, 10));
    }

    // ── CircuitBreaker ───────────────────────────────────────────────────────

    #[test]
    fn circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn record_success_on_closed_resets_failure_count() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig { failure_threshold: 3, ..Default::default() });
        cb.record_failure(0);
        cb.record_failure(0);
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_success(0);
        // Internal failure_count reset; one more failure shouldn't trip yet
        cb.record_failure(0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn record_failure_increments_and_trips_open() {
        let cfg = CircuitBreakerConfig { failure_threshold: 3, reset_after_ms: 30_000, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        cb.record_failure(1000);
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure(1001);
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure(1002);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn poll_open_before_reset_returns_open() {
        let cfg = CircuitBreakerConfig { failure_threshold: 1, reset_after_ms: 30_000, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        cb.record_failure(0);
        assert_eq!(cb.poll(29_999), CircuitState::Open);
    }

    #[test]
    fn poll_open_after_reset_returns_half_open() {
        let cfg = CircuitBreakerConfig { failure_threshold: 1, reset_after_ms: 30_000, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        cb.record_failure(0);
        assert_eq!(cb.poll(30_000), CircuitState::HalfOpen);
    }

    #[test]
    fn record_success_in_half_open_after_threshold_closes() {
        let cfg = CircuitBreakerConfig { failure_threshold: 1, reset_after_ms: 1, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        cb.record_failure(0);
        cb.poll(10); // → HalfOpen
        cb.record_success(10);
        assert_eq!(cb.state(), CircuitState::HalfOpen); // still need one more
        cb.record_success(11);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn record_failure_in_half_open_returns_to_open() {
        let cfg = CircuitBreakerConfig { failure_threshold: 1, reset_after_ms: 1, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        cb.record_failure(0);
        cb.poll(10); // → HalfOpen
        cb.record_failure(10);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn can_dispatch_false_in_open_true_otherwise() {
        let cfg = CircuitBreakerConfig { failure_threshold: 1, reset_after_ms: 30_000, half_open_success_threshold: 2 };
        let mut cb = CircuitBreaker::new(cfg);
        assert!(cb.can_dispatch(0)); // Closed
        cb.record_failure(0);
        assert!(!cb.can_dispatch(1_000)); // Open, not elapsed
        assert!(cb.can_dispatch(30_001)); // HalfOpen after elapsed
    }
}
