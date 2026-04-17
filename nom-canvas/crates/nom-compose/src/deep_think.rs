#![deny(unsafe_code)]

use crate::progress::{ComposeEvent, ProgressSink};
use nom_intent::{classify_with_react, react_chain};

/// One step in a chain-of-thought pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepThinkStep {
    pub hypothesis: String,
    pub evidence: Vec<String>,
    pub confidence: f32,
    pub counterevidence: Vec<String>,
    pub refined_from: Option<usize>,
}

/// Configures the deep-think streaming pipeline.
#[derive(Debug, Clone)]
pub struct DeepThinkConfig {
    pub max_steps: usize,
    pub beam_width: usize,
    pub token_budget: u32,
}

impl Default for DeepThinkConfig {
    fn default() -> Self {
        Self {
            max_steps: 5,
            beam_width: 3,
            token_budget: 2048,
        }
    }
}

/// Streaming wire that chains thinking steps and emits progress events.
pub struct DeepThinkStream {
    config: DeepThinkConfig,
}

impl DeepThinkStream {
    pub fn new(config: DeepThinkConfig) -> Self {
        Self { config }
    }

    /// Generates `config.max_steps` think steps using a ReAct reasoning loop,
    /// emitting `ComposeEvent::Progress` at each step and `ComposeEvent::Completed` at the end.
    pub fn think(&self, input_hash: u64, progress: &dyn ProgressSink) -> Vec<DeepThinkStep> {
        let input = format!("intent_{:016x}", input_hash);
        let mut steps = Vec::with_capacity(self.config.max_steps);

        for step_id in 0..self.config.max_steps {
            let hypothesis = format!("hypothesis_{}: {}", step_id, &input[..input.len().min(30)]);

            // Build evidence from previous step hypotheses (ReAct: each step observes prior)
            let prev_evidence: Vec<&str> = steps
                .iter()
                .map(|s: &DeepThinkStep| s.hypothesis.as_str())
                .collect();

            let confidence = if prev_evidence.is_empty() {
                0.5
            } else {
                let raw = classify_with_react(&hypothesis, &prev_evidence);
                (raw + step_id as f32 * 0.05).min(0.95)
            };

            let chain = react_chain(&hypothesis, &[input.as_str()], 1);
            let evidence: Vec<String> = chain.into_iter().map(|r| r.observation).collect();

            steps.push(DeepThinkStep {
                hypothesis,
                evidence,
                confidence,
                counterevidence: vec![],
                refined_from: if step_id > 0 { Some(step_id - 1) } else { None },
            });

            let pct = ((step_id + 1) as f32 / self.config.max_steps as f32) * 100.0;
            progress.emit(ComposeEvent::Progress {
                percent: pct,
                stage: format!("think_step_{}", step_id),
            });
        }

        // Signal completion with a zeroed artifact hash (no artifact stored by this wire).
        progress.emit(ComposeEvent::Completed {
            artifact_hash: [0u8; 32],
            byte_size: steps.len() as u64 * std::mem::size_of::<DeepThinkStep>() as u64,
        });

        steps
    }

    /// Run `config.beam_width` independent think chains in parallel and return
    /// them all.
    ///
    /// Each chain uses a hypothesis seeded with its beam index so beams diverge
    /// and produce different confidence trajectories.  A `ComposeEvent::Progress`
    /// is emitted after each beam completes; a final `ComposeEvent::Completed`
    /// is emitted once all beams are done.
    pub fn think_beam(
        &self,
        input_hash: u64,
        progress: &dyn ProgressSink,
    ) -> Vec<Vec<DeepThinkStep>> {
        let input = format!("intent_{:016x}", input_hash);
        let mut beams: Vec<Vec<DeepThinkStep>> = Vec::with_capacity(self.config.beam_width);

        for beam_i in 0..self.config.beam_width {
            let mut chain = Vec::with_capacity(self.config.max_steps);

            for step_id in 0..self.config.max_steps {
                // Include beam_i in hypothesis so each beam diverges
                let hypothesis = format!(
                    "beam{}_hypothesis_{}: {}",
                    beam_i,
                    step_id,
                    &input[..input.len().min(24)]
                );

                let prev_evidence: Vec<&str> = chain
                    .iter()
                    .map(|s: &DeepThinkStep| s.hypothesis.as_str())
                    .collect();

                let confidence = if prev_evidence.is_empty() {
                    0.5 + beam_i as f32 * 0.02
                } else {
                    let raw = classify_with_react(&hypothesis, &prev_evidence);
                    (raw + step_id as f32 * 0.05 + beam_i as f32 * 0.01).min(0.95)
                };

                let react_ev = react_chain(&hypothesis, &[input.as_str()], 1);
                let evidence: Vec<String> = react_ev.into_iter().map(|r| r.observation).collect();

                chain.push(DeepThinkStep {
                    hypothesis,
                    evidence,
                    confidence,
                    counterevidence: vec![],
                    refined_from: if step_id > 0 { Some(step_id - 1) } else { None },
                });
            }

            beams.push(chain);

            let pct = ((beam_i + 1) as f32 / self.config.beam_width as f32) * 100.0;
            progress.emit(ComposeEvent::Progress {
                percent: pct,
                stage: format!("beam_{}", beam_i),
            });
        }

        progress.emit(ComposeEvent::Completed {
            artifact_hash: [0u8; 32],
            byte_size: beams.iter().map(|c| c.len()).sum::<usize>() as u64
                * std::mem::size_of::<DeepThinkStep>() as u64,
        });

        beams
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::VecProgressSink;

    fn default_stream() -> DeepThinkStream {
        DeepThinkStream::new(DeepThinkConfig::default())
    }

    #[test]
    fn deep_think_produces_n_steps() {
        let stream = default_stream();
        let sink = VecProgressSink::new();
        let steps = stream.think(0xdeadbeef, &sink);
        assert_eq!(steps.len(), DeepThinkConfig::default().max_steps);
        for (i, step) in steps.iter().enumerate() {
            assert!(
                step.hypothesis.starts_with(&format!("hypothesis_{}:", i)),
                "step {} hypothesis should start with 'hypothesis_{i}:', got: {}",
                i,
                step.hypothesis
            );
        }
    }

    #[test]
    fn deep_think_emits_progress_events() {
        let stream = default_stream();
        let sink = VecProgressSink::new();
        let steps = stream.think(0x1234, &sink);
        let events = sink.take();

        // One Progress per step + one Completed at the end.
        let n = DeepThinkConfig::default().max_steps;
        assert_eq!(events.len(), n + 1);

        // All intermediate events are Progress.
        for event in &events[..n] {
            assert!(
                matches!(event, ComposeEvent::Progress { .. }),
                "expected Progress, got {:?}",
                event
            );
        }

        // Last event is Completed.
        assert!(
            matches!(events[n], ComposeEvent::Completed { .. }),
            "expected Completed, got {:?}",
            events[n]
        );

        // Final progress percent should be 100.
        if let ComposeEvent::Progress { percent, .. } = &events[n - 1] {
            assert!(
                (*percent - 100.0f32).abs() < 0.01,
                "last progress percent should be 100, got {}",
                percent
            );
        }

        // steps count matches config
        assert_eq!(steps.len(), n);
    }

    #[test]
    fn deep_think_beam_returns_multiple_chains() {
        let cfg = DeepThinkConfig {
            max_steps: 4,
            beam_width: 3,
            token_budget: 400,
        };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink = VecProgressSink::new();
        let beams = stream.think_beam(0xbeef_cafe, &sink);

        // Must return exactly beam_width chains.
        assert_eq!(
            beams.len(),
            cfg.beam_width,
            "number of chains must equal beam_width"
        );

        // Each chain must have exactly max_steps steps.
        for (i, chain) in beams.iter().enumerate() {
            assert_eq!(
                chain.len(),
                cfg.max_steps,
                "chain {} must have {} steps",
                i,
                cfg.max_steps
            );
        }

        // Different beams must differ (beam index is embedded in hypothesis).
        assert_ne!(beams[0], beams[1], "beam 0 and beam 1 should differ");
        assert_ne!(beams[1], beams[2], "beam 1 and beam 2 should differ");

        // Events: one Progress per beam + one Completed.
        let events = sink.take();
        assert_eq!(events.len(), cfg.beam_width + 1);
        assert!(matches!(
            events[cfg.beam_width],
            ComposeEvent::Completed { .. }
        ));
    }

    #[test]
    fn deep_think_beam_width_respected() {
        // beam_width=1 → single chain, same as think() with rotated seed.
        let cfg = DeepThinkConfig {
            max_steps: 3,
            beam_width: 1,
            token_budget: 300,
        };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink = VecProgressSink::new();
        let beams = stream.think_beam(0x1111, &sink);
        assert_eq!(beams.len(), 1);
        assert_eq!(beams[0].len(), cfg.max_steps);

        // beam_width=5 → five chains.
        let cfg5 = DeepThinkConfig {
            max_steps: 2,
            beam_width: 5,
            token_budget: 200,
        };
        let stream5 = DeepThinkStream::new(cfg5.clone());
        let sink5 = VecProgressSink::new();
        let beams5 = stream5.think_beam(0x2222, &sink5);
        assert_eq!(beams5.len(), 5);
        for chain in &beams5 {
            assert_eq!(chain.len(), cfg5.max_steps);
        }
    }

    #[test]
    fn deep_think_step_fields_are_correct() {
        let cfg = DeepThinkConfig {
            max_steps: 3,
            beam_width: 2,
            token_budget: 300,
        };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink = VecProgressSink::new();

        let steps = stream.think(0xabcd_ef01, &sink);

        // First step: no refined_from, confidence is the initial 0.5 (no prior evidence)
        assert_eq!(steps[0].refined_from, None);
        assert!(
            (steps[0].confidence - 0.5).abs() < 1e-6,
            "step 0 confidence should be 0.5, got {}",
            steps[0].confidence
        );
        assert_eq!(steps[0].counterevidence, Vec::<String>::new());

        // Subsequent steps: refined_from links are correct and confidence is in [0, 1]
        assert_eq!(steps[1].refined_from, Some(0));
        assert!(steps[1].confidence >= 0.0 && steps[1].confidence <= 1.0);

        assert_eq!(steps[2].refined_from, Some(1));
        assert!(steps[2].confidence >= 0.0 && steps[2].confidence <= 1.0);

        // Deterministic: same input → same output
        let stream2 = DeepThinkStream::new(cfg);
        let sink2 = VecProgressSink::new();
        let steps2 = stream2.think(0xabcd_ef01, &sink2);
        assert_eq!(steps, steps2, "same input must produce identical steps");
    }
}
