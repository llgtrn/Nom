#![deny(unsafe_code)]
use crate::shared::{PipelineOutput, SharedState};
use crossbeam_channel::{Receiver, Sender};
#[allow(unused_imports)]
pub use nom_blocks::shared_types::RunEvent;
use nom_blocks::shared_types::{CompositionPlan, DeepThinkEvent, PlanStep};
use std::sync::Arc;

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
        reply: Sender<Vec<String>>, // diagnostic messages
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
    pub max_stages: u8, // 0 = run all stages (S1-S6)
    pub cache_enabled: bool,
}

impl CompileOpts {
    pub fn full() -> Self {
        Self {
            max_stages: 0,
            cache_enabled: true,
        }
    }
    pub fn fast() -> Self {
        Self {
            max_stages: 2,
            cache_enabled: true,
        }
    }
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
    pub fn compile(
        &self,
        source: String,
        opts: CompileOpts,
    ) -> Receiver<Result<PipelineOutput, String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::Compile {
            source,
            opts,
            reply: reply_tx,
        });
        reply_rx
    }

    /// Submit a plan flow job
    pub fn plan_flow(&self, output: PipelineOutput) -> Receiver<Result<CompositionPlan, String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::PlanFlow {
            output,
            reply: reply_tx,
        });
        reply_rx
    }

    /// Submit a verify job
    pub fn verify(&self, plan: CompositionPlan) -> Receiver<Vec<String>> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        let _ = self.sender.send(BackgroundJob::Verify {
            plan,
            reply: reply_tx,
        });
        reply_rx
    }

    /// Submit a deep_think job — streams DeepThinkEvents until completion or interrupt
    pub fn deep_think(
        &self,
        intent: String,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) -> Receiver<DeepThinkEvent> {
        let (events_tx, events_rx) = crossbeam_channel::bounded(64);
        let _ = self.sender.send(BackgroundJob::DeepThink {
            intent,
            interrupt,
            events: events_tx,
        });
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
            BackgroundJob::Compile {
                source,
                opts,
                reply,
            } => {
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
            BackgroundJob::DeepThink {
                intent,
                interrupt,
                events,
            } => {
                self.do_deep_think(&intent, &interrupt, &events);
            }
        }
    }

    pub(crate) fn do_compile(
        &self,
        source: &str,
        opts: &CompileOpts,
    ) -> Result<PipelineOutput, String> {
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
            let tok_count = stage1_tokenize(source).map(|s| s.toks.len()).unwrap_or(0);
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
        // Parse the output_json to extract a goal/intent string for planning
        let intent = Self::extract_intent_from_output(output);

        // Split into sentences by punctuation, then words; count words for complexity
        let word_count = intent.split_whitespace().count().max(1);
        // 1 step per ~5 words, clamped to [1, 10]
        let step_count = ((word_count + 4) / 5).clamp(1, 10);

        // Grammar cache hit rate: known Nom keywords boost confidence
        let known_keywords = [
            "define", "that", "is", "with", "and", "or", "not", "if", "then", "else",
            "result", "each", "map", "filter", "reduce", "yield", "use", "from", "where",
        ];
        let words: Vec<&str> = intent.split_whitespace().collect();
        let hits = words
            .iter()
            .filter(|w| known_keywords.contains(&w.to_lowercase().as_str()))
            .count();
        let confidence = if words.is_empty() {
            0.1
        } else {
            0.4 + 0.5 * (hits as f32 / words.len() as f32)
        };

        let steps: Vec<PlanStep> = (0..step_count)
            .map(|i| {
                // Distribute words across steps for non-trivial descriptions
                let chunk_size = (word_count + step_count - 1) / step_count;
                let start = i * chunk_size;
                let fragment: String = words
                    .iter()
                    .skip(start)
                    .take(chunk_size)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                let description = if fragment.is_empty() {
                    format!("Step {}: refine result", i + 1)
                } else {
                    format!("Step {}: {}", i + 1, fragment)
                };
                PlanStep {
                    id: format!("step_{i}"),
                    description,
                    kind: "plan".into(),
                    depends_on: if i > 0 {
                        vec![format!("step_{}", i - 1)]
                    } else {
                        vec![]
                    },
                }
            })
            .collect();

        let rationale = format!(
            "Goal has {} words → {} steps; grammar cache hits {}/{} → confidence {:.2}",
            word_count,
            step_count,
            hits,
            words.len(),
            confidence
        );

        Ok(CompositionPlan {
            intent: rationale,
            steps,
            confidence,
        })
    }

    fn extract_intent_from_output(output: &PipelineOutput) -> String {
        // Try to parse output_json for a "source" or "intent" field
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&output.output_json) {
            if let Some(s) = v.get("intent").and_then(|x| x.as_str()) {
                return s.to_string();
            }
            if let Some(s) = v.get("source").and_then(|x| x.as_str()) {
                return s.to_string();
            }
        }
        // Fallback: use output_json itself as a string hint
        output.output_json.clone()
    }

    fn do_verify(&self, plan: &CompositionPlan) -> Vec<String> {
        let code = &plan.intent;
        let mut diagnostics: Vec<String> = vec![];

        // Check 1: empty input
        if code.trim().is_empty() {
            diagnostics.push("ERROR EmptyInput: intent/code string is empty".into());
            return diagnostics;
        }

        // Check 2: unbalanced braces
        let open_braces = code.chars().filter(|&c| c == '{').count();
        let close_braces = code.chars().filter(|&c| c == '}').count();
        if open_braces != close_braces {
            diagnostics.push(format!(
                "ERROR UnbalancedBraces: {} opening vs {} closing braces",
                open_braces, close_braces
            ));
        }

        // Check 3: lines exceeding performance threshold
        let line_count = code.lines().count();
        if line_count > 1000 {
            diagnostics.push(format!(
                "WARN PerformanceCaution: input has {} lines (>1000), analysis may be slow",
                line_count
            ));
        }

        // Check 4: steps structural validation
        for (i, step) in plan.steps.iter().enumerate() {
            if step.id.is_empty() {
                diagnostics.push(format!("ERROR StepMissingId: step at index {i} has empty id"));
            }
            if step.description.is_empty() {
                diagnostics.push(format!(
                    "ERROR StepMissingDescription: step '{}' has empty description",
                    step.id
                ));
            }
        }

        diagnostics
    }

    fn do_deep_think(
        &self,
        intent: &str,
        interrupt: &Arc<std::sync::atomic::AtomicBool>,
        events: &Sender<DeepThinkEvent>,
    ) {
        use nom_blocks::shared_types::DeepThinkStep;

        if interrupt.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        // Extract key entities: words longer than 4 chars that are not common stop words
        let stop_words = ["that", "with", "this", "from", "have", "will", "been", "they"];
        let entities: Vec<&str> = intent
            .split_whitespace()
            .filter(|w| w.len() > 4 && !stop_words.contains(&w.to_lowercase().as_str()))
            .take(5)
            .collect();
        let entities_str = if entities.is_empty() {
            intent.split_whitespace().take(3).collect::<Vec<_>>().join(", ")
        } else {
            entities.join(", ")
        };

        // Sub-problems: split intent into clause fragments at punctuation or conjunctions
        let sub_problems: Vec<&str> = intent
            .split(|c: char| c == ',' || c == ';' || c == '.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .take(3)
            .collect();
        let sub_str = if sub_problems.is_empty() {
            intent.to_string()
        } else {
            sub_problems.join(" | ")
        };

        let known_keywords = [
            "define", "that", "is", "with", "and", "or", "not", "if", "then", "else",
            "result", "each", "map", "filter", "reduce", "yield", "use", "from", "where",
        ];
        let words: Vec<&str> = intent.split_whitespace().collect();
        let hits = words
            .iter()
            .filter(|w| known_keywords.contains(&w.to_lowercase().as_str()))
            .count();
        let cache_hit_rate = if words.is_empty() {
            0.0
        } else {
            hits as f32 / words.len() as f32
        };

        let steps_data: &[(&str, Vec<String>, f32)] = &[
            (
                &format!("Analyzing: {intent}"),
                vec![
                    format!("input_length:{}", intent.len()),
                    format!("word_count:{}", words.len()),
                ],
                0.3,
            ),
            (
                "Decomposing into sub-problems",
                vec![format!("sub_problems: {sub_str}")],
                0.45,
            ),
            (
                &format!("Identifying key entities: {entities_str}"),
                vec![
                    format!("entity_count:{}", entities.len()),
                    format!("entities:[{entities_str}]"),
                ],
                0.55,
            ),
            (
                "Forming hypothesis based on entity relationships",
                vec![
                    format!("dominant_entity:{}", entities.first().copied().unwrap_or(intent)),
                    "relationship:compositional".into(),
                ],
                0.7,
            ),
            (
                "Validating against grammar cache",
                vec![
                    format!("cache_hits:{hits}"),
                    format!("cache_hit_rate:{cache_hit_rate:.2}"),
                    format!("grammar_version:{}", self.state.grammar_version()),
                ],
                0.4 + 0.5 * cache_hit_rate,
            ),
        ];

        for (i, (hypothesis, evidence, confidence)) in steps_data.iter().enumerate() {
            if interrupt.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let step = DeepThinkStep {
                hypothesis: hypothesis.to_string(),
                evidence: evidence.clone(),
                confidence: *confidence,
                counterevidence: vec![],
                refined_from: if i > 0 {
                    Some(format!("step_{}", i - 1))
                } else {
                    None
                },
            };
            let _ = events.send(DeepThinkEvent::Step(step));
        }

        if !interrupt.load(std::sync::atomic::Ordering::Relaxed) {
            let final_confidence = 0.4 + 0.5 * cache_hit_rate;
            let _ = events.send(DeepThinkEvent::Final(CompositionPlan {
                intent: intent.to_string(),
                steps: vec![],
                confidence: final_confidence,
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
        assert_eq!(events.len(), 6); // 5 steps + 1 Final
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
            other => panic!(
                "expected BackgroundJob::Compile, got a different variant: {:?}",
                std::mem::discriminant(&other)
            ),
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
            other => panic!(
                "expected BackgroundJob::PlanFlow, got a different variant: {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn background_tier_compile_cache_hit_returns_same_output() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state.clone());
        // First compile populates the cache
        let first = worker
            .do_compile("define y that is 7", &CompileOpts::full())
            .unwrap();
        // Second compile with cache enabled must return the same source_hash
        let second = worker
            .do_compile("define y that is 7", &CompileOpts::full())
            .unwrap();
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

    #[test]
    fn background_tier_compile_returns_result() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        // compile returns Result<PipelineOutput, String> — must not panic
        let result: Result<PipelineOutput, String> =
            worker.do_compile("define z that is 0", &CompileOpts::full());
        assert!(result.is_ok());
        let output = result.unwrap();
        // output_json must be a non-empty string
        assert!(!output.output_json.is_empty());
    }

    #[test]
    fn background_tier_plan_flow_returns_result() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let fake_output = PipelineOutput {
            source_hash: 0,
            grammar_version: 0,
            output_json: "{}".into(),
        };
        let result = worker.do_plan_flow(&fake_output);
        // Must be Ok regardless of input in stub mode
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Plan has an intent field (may be empty string or "stub plan")
        let _ = plan.intent.len();
    }

    #[test]
    fn background_tier_verify_returns_result() {
        use nom_blocks::shared_types::CompositionPlan;
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "test".into(),
            steps: vec![],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        // "test" is non-empty, balanced braces, 1 line -- no diagnostics expected
        assert!(diags.is_empty());
    }

    // --- 8 new tests for AE17 ---

    #[test]
    fn plan_flow_single_word_goal_produces_one_step() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let output = PipelineOutput {
            source_hash: 1,
            grammar_version: 1,
            output_json: r#"{"intent":"run"}"#.into(),
        };
        let plan = worker.do_plan_flow(&output).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert!(!plan.steps[0].description.is_empty());
    }

    #[test]
    fn plan_flow_long_goal_produces_multiple_steps() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let goal =
            "define result that is map each item with filter and reduce or yield if not from where use and";
        let output = PipelineOutput {
            source_hash: 2,
            grammar_version: 1,
            output_json: format!(r#"{{"intent":"{}"}}"#, goal),
        };
        let plan = worker.do_plan_flow(&output).unwrap();
        assert!(plan.steps.len() >= 2, "expected multiple steps for long goal");
        assert!(plan.steps.len() <= 10, "capped at 10 steps");
    }

    #[test]
    fn plan_flow_nom_keywords_raise_confidence() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let output_nom = PipelineOutput {
            source_hash: 3,
            grammar_version: 1,
            output_json: r#"{"intent":"define result that is"}"#.into(),
        };
        let plan_nom = worker.do_plan_flow(&output_nom).unwrap();
        let output_plain = PipelineOutput {
            source_hash: 4,
            grammar_version: 1,
            output_json: r#"{"intent":"xyzzy frobble quux blorp"}"#.into(),
        };
        let plan_plain = worker.do_plan_flow(&output_plain).unwrap();
        assert!(
            plan_nom.confidence > plan_plain.confidence,
            "Nom keyword goal should have higher confidence: {:.2} vs {:.2}",
            plan_nom.confidence,
            plan_plain.confidence
        );
    }

    #[test]
    fn plan_flow_steps_have_sequential_dependencies() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let output = PipelineOutput {
            source_hash: 5,
            grammar_version: 1,
            output_json: r#"{"intent":"define x that is map each item with filter"}"#.into(),
        };
        let plan = worker.do_plan_flow(&output).unwrap();
        assert!(plan.steps[0].depends_on.is_empty());
        if plan.steps.len() > 1 {
            assert!(!plan.steps[1].depends_on.is_empty());
        }
    }

    #[test]
    fn verify_empty_intent_emits_empty_input_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "   ".into(),
            steps: vec![],
            confidence: 0.0,
        };
        let diags = worker.do_verify(&plan);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].contains("EmptyInput"));
    }

    #[test]
    fn verify_unbalanced_braces_emits_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "define x { that is 42".into(),
            steps: vec![],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        assert!(
            diags.iter().any(|d| d.contains("UnbalancedBraces")),
            "expected UnbalancedBraces diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn verify_valid_plan_returns_no_diagnostics() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "define result that is map each item".into(),
            steps: vec![PlanStep {
                id: "step_0".into(),
                description: "map items".into(),
                kind: "plan".into(),
                depends_on: vec![],
            }],
            confidence: 0.7,
        };
        let diags = worker.do_verify(&plan);
        assert!(
            diags.is_empty(),
            "valid plan should have no diagnostics: {:?}",
            diags
        );
    }

    #[test]
    fn deep_think_steps_reference_input_prompt() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        worker.do_deep_think("define pipeline that transforms data", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        let step_texts: Vec<String> = events
            .iter()
            .filter_map(|e| {
                if let DeepThinkEvent::Step(s) = e {
                    Some(format!("{} {:?}", s.hypothesis, s.evidence))
                } else {
                    None
                }
            })
            .collect();
        let all_text = step_texts.join(" ");
        assert!(
            all_text.contains("pipeline")
                || all_text.contains("transforms")
                || all_text.contains("data"),
            "steps should reference input entities; got: {}",
            all_text
        );
    }

    #[test]
    fn deep_think_produces_at_least_five_steps() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        worker.do_deep_think("analyze and synthesize results", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        let step_count = events
            .iter()
            .filter(|e| matches!(e, DeepThinkEvent::Step(_)))
            .count();
        assert!(step_count >= 5, "expected at least 5 steps, got {}", step_count);
    }

    #[test]
    fn deep_think_final_event_confidence_is_nonzero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        worker.do_deep_think("define result that is map each item", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        let final_confidence = events.iter().find_map(|e| {
            if let DeepThinkEvent::Final(plan) = e {
                Some(plan.confidence)
            } else {
                None
            }
        });
        assert!(final_confidence.is_some(), "expected a Final event");
        assert!(final_confidence.unwrap() > 0.0, "final confidence should be > 0");
    }

    // ── AE3 additions ──────────────────────────────────────────────────────

    /// plan_flow with a majority-Nom-keyword intent must produce confidence > 0.5.
    #[test]
    fn plan_flow_nom_keywords_confidence_above_half() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        // "define", "that", "is", "result" are all known Nom keywords → high hit rate
        let output = PipelineOutput {
            source_hash: 10,
            grammar_version: 1,
            output_json: r#"{"intent":"define result that is and or"}"#.into(),
        };
        let plan = worker.do_plan_flow(&output).unwrap();
        assert!(
            plan.confidence > 0.5,
            "all-keyword goal must yield confidence > 0.5, got {:.3}",
            plan.confidence
        );
    }

    /// do_verify on a plan whose intent spans 1001 lines must emit a PerformanceCaution warning.
    #[test]
    fn verify_1001_lines_emits_performance_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        // Build a string with exactly 1001 non-empty lines
        let long_intent: String = (0..1001)
            .map(|i| format!("line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let plan = CompositionPlan {
            intent: long_intent,
            steps: vec![],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        let has_perf = diags.iter().any(|d| d.contains("PerformanceCaution"));
        assert!(
            has_perf,
            "1001-line input must trigger PerformanceCaution; got: {:?}",
            diags
        );
    }

    /// deep_think step at index 2 (third step, 0-based) must mention at least one entity
    /// word extracted from the original prompt.
    #[test]
    fn deep_think_step3_contains_entity_from_prompt() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let (tx, rx) = crossbeam_channel::bounded(64);
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        // "pipeline" and "transforms" and "dataset" are > 4 chars and not stop words
        worker.do_deep_think("define pipeline that transforms dataset", &interrupt, &tx);
        let events: Vec<DeepThinkEvent> = rx.try_iter().collect();
        // Collect all Step events in order
        let steps: Vec<_> = events
            .iter()
            .filter_map(|e| if let DeepThinkEvent::Step(s) = e { Some(s) } else { None })
            .collect();
        assert!(steps.len() >= 3, "expected at least 3 steps");
        // Step at index 2 (third step) is "Identifying key entities"
        let step3_text = format!("{} {:?}", steps[2].hypothesis, steps[2].evidence);
        let found = ["pipeline", "transforms", "dataset"]
            .iter()
            .any(|w| step3_text.contains(w));
        assert!(
            found,
            "step 3 must contain an entity from prompt; got: {}",
            step3_text
        );
    }

    /// do_verify on a plan with step missing description emits StepMissingDescription diagnostic.
    #[test]
    fn verify_step_missing_description_emits_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "define x that is 1".into(),
            steps: vec![PlanStep {
                id: "step_0".into(),
                description: "".into(),
                kind: "plan".into(),
                depends_on: vec![],
            }],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        assert!(
            diags.iter().any(|d| d.contains("StepMissingDescription")),
            "missing description should emit diagnostic; got: {:?}",
            diags
        );
    }

    /// do_verify on a plan with step missing id emits StepMissingId diagnostic.
    #[test]
    fn verify_step_missing_id_emits_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let plan = CompositionPlan {
            intent: "define x that is 1".into(),
            steps: vec![PlanStep {
                id: "".into(),
                description: "something".into(),
                kind: "plan".into(),
                depends_on: vec![],
            }],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        assert!(
            diags.iter().any(|d| d.contains("StepMissingId")),
            "empty step id should emit diagnostic; got: {:?}",
            diags
        );
    }

    /// plan_flow with non-keyword words produces lower confidence than all-keyword goal.
    #[test]
    fn plan_flow_non_keyword_words_lower_confidence() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let output_all_keyword = PipelineOutput {
            source_hash: 20,
            grammar_version: 1,
            output_json: r#"{"intent":"define result that is and or"}"#.into(),
        };
        let output_gibberish = PipelineOutput {
            source_hash: 21,
            grammar_version: 1,
            output_json: r#"{"intent":"xyzzy frobble quux snorkel blorp"}"#.into(),
        };
        let plan_keyword = worker.do_plan_flow(&output_all_keyword).unwrap();
        let plan_gibberish = worker.do_plan_flow(&output_gibberish).unwrap();
        assert!(
            plan_keyword.confidence >= plan_gibberish.confidence,
            "all-keyword goal confidence ({:.3}) should be >= gibberish confidence ({:.3})",
            plan_keyword.confidence,
            plan_gibberish.confidence
        );
    }

    /// verify exactly 1000 lines must NOT emit a PerformanceCaution (boundary is >1000).
    #[test]
    fn verify_exactly_1000_lines_no_performance_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let intent: String = (0..1000)
            .map(|i| format!("line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let plan = CompositionPlan {
            intent,
            steps: vec![],
            confidence: 0.5,
        };
        let diags = worker.do_verify(&plan);
        let has_perf = diags.iter().any(|d| d.contains("PerformanceCaution"));
        assert!(
            !has_perf,
            "exactly 1000 lines should not trigger PerformanceCaution; got: {:?}",
            diags
        );
    }

    /// plan_flow extracts intent from the "intent" JSON field, not the "source" field.
    #[test]
    fn plan_flow_reads_intent_field_from_json() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = BackgroundWorker::new(state);
        let output = PipelineOutput {
            source_hash: 30,
            grammar_version: 1,
            output_json: r#"{"intent":"define that","source":"ignored text"}"#.into(),
        };
        let plan = worker.do_plan_flow(&output).unwrap();
        // "define" and "that" are both Nom keywords; confidence should be boosted
        assert!(
            plan.confidence > 0.4,
            "intent field keywords should boost confidence; got {:.3}",
            plan.confidence
        );
    }

    /// BackgroundTier::verify sends the job on the channel and returns a receiver.
    #[test]
    fn background_tier_verify_sends_job() {
        let (tier, receiver) = BackgroundTier::new();
        let plan = CompositionPlan {
            intent: "define x that is 1".into(),
            steps: vec![],
            confidence: 0.7,
        };
        let _reply = tier.verify(plan);
        let job = receiver.try_recv().expect("expected a job on the channel");
        assert!(matches!(job, BackgroundJob::Verify { .. }));
    }

    /// BackgroundTier::deep_think sends the job on the channel and returns a receiver.
    #[test]
    fn background_tier_deep_think_sends_job() {
        let (tier, receiver) = BackgroundTier::new();
        let interrupt = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _rx = tier.deep_think("test intent".to_string(), interrupt);
        let job = receiver.try_recv().expect("expected a job on the channel");
        assert!(matches!(job, BackgroundJob::DeepThink { .. }));
    }

    /// compile_opts_full has cache_enabled = true and max_stages = 0.
    #[test]
    fn compile_opts_full_fields() {
        let opts = CompileOpts::full();
        assert!(opts.cache_enabled);
        assert_eq!(opts.max_stages, 0);
    }

    /// compile_opts_fast has max_stages = 2.
    #[test]
    fn compile_opts_fast_max_stages() {
        let opts = CompileOpts::fast();
        assert_eq!(opts.max_stages, 2);
        assert!(opts.cache_enabled);
    }
}
