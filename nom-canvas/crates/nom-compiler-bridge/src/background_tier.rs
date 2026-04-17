#![deny(unsafe_code)]
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::shared::{SharedState, PipelineOutput};
use nom_blocks::shared_types::{CompositionPlan, DeepThinkEvent};
#[allow(unused_imports)]
pub use nom_blocks::shared_types::RunEvent;

/// Background job variants (Refly BullMQ pattern ported to Rust crossbeam channels)
pub enum BackgroundJob {
    /// Compile source text through full nom-compiler pipeline
    Compile {
        source: String,
        opts: CompileOpts,
        reply: Sender<Result<PipelineOutput, String>>,
    },
    /// Plan a composition flow from pipeline output
    PlanFlow {
        output: PipelineOutput,
        reply: Sender<Result<CompositionPlan, String>>,
    },
    /// Verify a composition plan
    Verify {
        plan: CompositionPlan,
        reply: Sender<Vec<String>>,  // diagnostic messages
    },
    /// Deep think: scored ReAct loop, max 10 steps
    DeepThink {
        intent: String,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
        events: Sender<DeepThinkEvent>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CompileOpts {
    pub max_stages: u8,           // 0 = run all stages (S1-S6)
    pub cache_enabled: bool,
}

impl CompileOpts {
    pub fn full() -> Self { Self { max_stages: 0, cache_enabled: true } }
    pub fn fast() -> Self { Self { max_stages: 2, cache_enabled: true } }
}

pub struct BackgroundTier {
    sender: Sender<BackgroundJob>,
}

impl BackgroundTier {
    pub fn new() -> (Self, Receiver<BackgroundJob>) {
        let (sender, receiver) = crossbeam_channel::bounded(256);
        (Self { sender }, receiver)
    }

    /// Submit a compile job. Returns immediately — result delivered via reply channel.
    pub fn compile(&self, source: String, opts: CompileOpts) -> Receiver<Result<PipelineOutput, String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::Compile { source, opts, reply: reply_tx });
        reply_rx
    }

    /// Submit a plan flow job
    pub fn plan_flow(&self, output: PipelineOutput) -> Receiver<Result<CompositionPlan, String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::PlanFlow { output, reply: reply_tx });
        reply_rx
    }

    /// Submit a verify job
    pub fn verify(&self, plan: CompositionPlan) -> Receiver<Vec<String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::Verify { plan, reply: reply_tx });
        reply_rx
    }

    /// Submit a deep_think job — streams DeepThinkEvents until completion or interrupt
    pub fn deep_think(
        &self,
        intent: String,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) -> Receiver<DeepThinkEvent> {
        let (events_tx, events_rx) = crossbeam_channel::bounded(64);
        let _ = self.sender.send(BackgroundJob::DeepThink { intent, interrupt, events: events_tx });
        events_rx
    }
}

impl Default for BackgroundTier {
    fn default() -> Self {
        Self::new().0
    }
}

/// BackgroundTierOps — Arc-owned accessor for background-tier operations (>100ms, may block)
pub struct BackgroundTierOps {
    shared: Arc<SharedState>,
}

impl BackgroundTierOps {
    pub fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }

    /// Plan pipeline steps from a source string — returns non-empty lines as steps
    pub fn plan_pipeline(&self, source: &str) -> Vec<String> {
        source
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect()
    }

    /// Submit a compile job and return the result synchronously (blocks)
    pub fn compile_sync(&self, source: &str) -> Result<PipelineOutput, String> {
        let worker = BackgroundWorker::new(self.shared.clone());
        worker.do_compile(source, &CompileOpts::full())
    }
}

/// Worker that processes background jobs (runs on a thread pool)
pub struct BackgroundWorker {
    state: Arc<SharedState>,
}

impl BackgroundWorker {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }

    pub fn run(self, receiver: Receiver<BackgroundJob>) {
        while let Ok(job) = receiver.recv() {
            self.handle(job);
        }
    }

    fn handle(&self, job: BackgroundJob) {
        match job {
            BackgroundJob::Compile { source, opts, reply } => {
                let result = self.do_compile(&source, &opts);
                let _ = reply.send(result);
            }
            BackgroundJob::PlanFlow { output, reply } => {
                let result = self.do_plan_flow(&output);
                let _ = reply.send(result);
            }
            BackgroundJob::Verify { plan, reply } => {
                let diagnostics = self.do_verify(&plan);
                let _ = reply.send(diagnostics);
            }
            BackgroundJob::DeepThink { intent, interrupt, events } => {
                self.do_deep_think(&intent, &interrupt, &events);
            }
        }
    }

    pub(crate) fn do_compile(&self, source: &str, opts: &CompileOpts) -> Result<PipelineOutput, String> {
        let version = self.state.grammar_version();
        let cache_key = SharedState::compile_cache_key(source, version);

        if opts.cache_enabled {
            if let Some(cached) = self.state.get_cached_compile(cache_key) {
                return Ok(cached);
            }
        }

        // With compiler feature: run stage1 tokenize from nom-concept
        #[cfg(feature = "compiler")]
        {
            use nom_concept::stage1_tokenize;
            let tok_count = stage1_tokenize(source)
                .map(|s| s.toks.len())
                .unwrap_or(0);
            let output = PipelineOutput {
                source_hash: cache_key,
                grammar_version: version,
                output_json: format!(
                    "{{\"source_len\":{},\"tok_count\":{}}}",
                    source.len(),
                    tok_count
                ),
            };
            self.state.cache_compile_result(cache_key, output.clone());
            Ok(output)
        }

        #[cfg(not(feature = "compiler"))]
        {
            let output = PipelineOutput {
                source_hash: cache_key,
                grammar_version: version,
                output_json: format!("{{\"source\":\"{}\",\"stub\":true}}", source.len()),
            };
            if opts.cache_enabled {
                self.state.cache_compile_result(cache_key, output.clone());
            }
            Ok(output)
        }
    }

    fn do_plan_flow(&self, output: &PipelineOutput) -> Result<CompositionPlan, String> {
        let _ = output;
        Ok(CompositionPlan {
            intent: "stub plan".into(),
            steps: vec![],
            confidence: 0.0,
        })
    }

    fn do_verify(&self, _plan: &CompositionPlan) -> Vec<String> {
        // Wave C: use nom-verifier or nom-diagnostics
        vec![]
    }

    fn do_deep_think(
        &self,
        intent: &str,
        interrupt: &Arc<std::sync::atomic::AtomicBool>,
        events: &Sender<DeepThinkEvent>,
    ) {
        use nom_blocks::shared_types::DeepThinkStep;

        // Stub: emit 3 steps then Final
        for i in 0..3 {
            if interrupt.load(std::sync::atomic::Ordering::Relaxed) { break; }
            let step = DeepThinkStep {
                hypothesis: format!("Hypothesis {i}: exploring intent '{intent}'"),
                evidence: vec![format!("evidence_{i}_a"), format!("evidence_{i}_b")],
                confidence: 0.3 + (i as f32) * 0.2,
                counterevidence: vec![],
                refined_from: if i > 0 { Some(format!("step_{}", i - 1)) } else { None },
            };
            let _ = events.send(DeepThinkEvent::Step(step));
        }

        if !interrupt.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = events.send(DeepThinkEvent::Final(CompositionPlan {
                intent: intent.to_string(),
                steps: vec![],
                confidence: 0.9,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_tier_compile_stub() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state.clone());
        let result = worker.do_compile("define x that is 42", &CompileOpts::full());
        assert!(result.is_ok());
    }

    #[test]
    fn background_tier_deep_think_steps() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        worker.do_deep_think("summarize document", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        assert_eq!(events.len(), 4); // 3 steps + 1 Final
        assert!(matches!(events.last(), Some(DeepThinkEvent::Final(_))));
    }

    #[test]
    fn background_tier_interrupt_stops_deep_think() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(true)); // pre-interrupted
        worker.do_deep_think("test", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        assert!(events.is_empty(), "interrupted before any events");
    }

    #[test]
    fn background_tier_ops_plan_pipeline() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let ops = BackgroundTierOps::new(state);
        let steps = ops.plan_pipeline("define x\n\nthat is 42\nresult");
        assert_eq!(steps, vec!["define x", "that is 42", "result"]);
    }

    #[test]
    fn background_tier_ops_compile_sync() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let ops = BackgroundTierOps::new(state);
        let result = ops.compile_sync("define x that is 42");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output_json.contains("stub") || output.output_json.contains("source"));
    }

    #[test]
    fn background_tier_compile_sends_job() {
        let (tier, receiver) = BackgroundTier::new();
        let opts = CompileOpts::full();
        let source = "define x that is 42".to_string();
        let _reply = tier.compile(source.clone(), opts);
        // The job must be immediately visible in the receiver
        let job = receiver.try_recv().expect("expected a job on the channel");
        match job {
            BackgroundJob::Compile { source: s, .. } => {
                assert_eq!(s, source);
            }
            other => panic!("expected BackgroundJob::Compile, got a different variant: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn background_tier_plan_flow_sends_job() {
        let (tier, receiver) = BackgroundTier::new();
        let output = PipelineOutput {
            source_hash: 99,
            grammar_version: 1,
            output_json: r#"{"stub":true}"#.into(),
        };
        let _reply = tier.plan_flow(output.clone());
        let job = receiver.try_recv().expect("expected a job on the channel");
        match job {
            BackgroundJob::PlanFlow { output: o, .. } => {
                assert_eq!(o.source_hash, output.source_hash);
                assert_eq!(o.grammar_version, output.grammar_version);
            }
            other => panic!("expected BackgroundJob::PlanFlow, got a different variant: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn background_tier_compile_cache_hit_returns_same_output() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state.clone());
        // First compile populates the cache
        let first = worker.do_compile("define y that is 7", &CompileOpts::full()).unwrap();
        // Second compile with cache enabled must return the same source_hash
        let second = worker.do_compile("define y that is 7", &CompileOpts::full()).unwrap();
        assert_eq!(first.source_hash, second.source_hash);
        assert_eq!(first.grammar_version, second.grammar_version);
    }

    #[test]
    fn background_tier_creates_from_shared() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let ops = BackgroundTierOps::new(state.clone());
        // Verify construction works and Arc refcount is at least 2 (original + ops)
        assert!(Arc::strong_count(&state) >= 2);
        drop(ops);
    }

    #[test]
    fn background_tier_compile_stub_returns_ok() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let result = worker.do_compile("", &CompileOpts::fast());
        // Stub must return Ok even for empty source
        assert!(result.is_ok());
    }
}
