#![deny(unsafe_code)]

use crate::progress::{ComposeEvent, ProgressSink};

/// One step in a chain-of-thought pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkStep {
    pub step_id: u32,
    pub prompt_hash: u64,
    pub output_hash: u64,
    pub token_count: u32,
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

    /// Generates `config.max_steps` think steps, emitting `ComposeEvent::Progress` at each step
    /// and `ComposeEvent::Completed` at the end.
    pub fn think(&self, input_hash: u64, progress: &dyn ProgressSink) -> Vec<ThinkStep> {
        let mut steps = Vec::with_capacity(self.config.max_steps);
        let tokens_per_step = self.config.token_budget / self.config.max_steps as u32;

        for i in 0..self.config.max_steps {
            let step_id = i as u32;
            let prompt_hash = input_hash
                .wrapping_mul(step_id as u64 + 1)
                .rotate_left(17);
            let output_hash = prompt_hash ^ (step_id as u64 * 0xcafe);

            steps.push(ThinkStep {
                step_id,
                prompt_hash,
                output_hash,
                token_count: tokens_per_step,
            });

            let pct = ((i + 1) as f32 / self.config.max_steps as f32) * 100.0;
            progress.emit(ComposeEvent::Progress {
                percent: pct,
                stage: format!("think_step_{}", step_id),
            });
        }

        // Signal completion with a zeroed artifact hash (no artifact stored by this wire).
        progress.emit(ComposeEvent::Completed {
            artifact_hash: [0u8; 32],
            byte_size: steps.len() as u64 * std::mem::size_of::<ThinkStep>() as u64,
        });

        steps
    }

    /// Run `config.beam_width` independent think chains in parallel and return
    /// them all.
    ///
    /// Each chain uses a seed derived from `input_hash` by rotating left by
    /// `beam_i * 7` bits.  A `ComposeEvent::Progress` is emitted after each
    /// beam completes; a final `ComposeEvent::Completed` is emitted once all
    /// beams are done.
    pub fn think_beam(&self, input_hash: u64, progress: &dyn ProgressSink) -> Vec<Vec<ThinkStep>> {
        let mut beams: Vec<Vec<ThinkStep>> = Vec::with_capacity(self.config.beam_width);
        let tokens_per_step = self.config.token_budget / self.config.max_steps as u32;

        for beam_i in 0..self.config.beam_width {
            let seed = input_hash.rotate_left(beam_i as u32 * 7);
            let mut chain = Vec::with_capacity(self.config.max_steps);

            for i in 0..self.config.max_steps {
                let step_id = i as u32;
                let prompt_hash = seed
                    .wrapping_mul(step_id as u64 + 1)
                    .rotate_left(17);
                let output_hash = prompt_hash ^ (step_id as u64 * 0xcafe);
                chain.push(ThinkStep {
                    step_id,
                    prompt_hash,
                    output_hash,
                    token_count: tokens_per_step,
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
                * std::mem::size_of::<ThinkStep>() as u64,
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
            assert_eq!(step.step_id, i as u32);
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
        let cfg = DeepThinkConfig { max_steps: 4, beam_width: 3, token_budget: 400 };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink = VecProgressSink::new();
        let beams = stream.think_beam(0xbeef_cafe, &sink);

        // Must return exactly beam_width chains.
        assert_eq!(beams.len(), cfg.beam_width, "number of chains must equal beam_width");

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

        // Different beams must differ (seeds are rotated differently).
        assert_ne!(beams[0], beams[1], "beam 0 and beam 1 should differ");
        assert_ne!(beams[1], beams[2], "beam 1 and beam 2 should differ");

        // Events: one Progress per beam + one Completed.
        let events = sink.take();
        assert_eq!(events.len(), cfg.beam_width + 1);
        assert!(matches!(events[cfg.beam_width], ComposeEvent::Completed { .. }));
    }

    #[test]
    fn deep_think_beam_width_respected() {
        // beam_width=1 → single chain, same as think() with rotated seed.
        let cfg = DeepThinkConfig { max_steps: 3, beam_width: 1, token_budget: 300 };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink = VecProgressSink::new();
        let beams = stream.think_beam(0x1111, &sink);
        assert_eq!(beams.len(), 1);
        assert_eq!(beams[0].len(), cfg.max_steps);

        // beam_width=5 → five chains.
        let cfg5 = DeepThinkConfig { max_steps: 2, beam_width: 5, token_budget: 200 };
        let stream5 = DeepThinkStream::new(cfg5.clone());
        let sink5 = VecProgressSink::new();
        let beams5 = stream5.think_beam(0x2222, &sink5);
        assert_eq!(beams5.len(), 5);
        for chain in &beams5 {
            assert_eq!(chain.len(), cfg5.max_steps);
        }
    }

    #[test]
    fn deep_think_step_hashes_are_deterministic() {
        let cfg = DeepThinkConfig { max_steps: 3, beam_width: 2, token_budget: 300 };
        let stream = DeepThinkStream::new(cfg.clone());
        let sink1 = VecProgressSink::new();
        let sink2 = VecProgressSink::new();

        let run1 = stream.think(0xabcd_ef01, &sink1);
        let stream2 = DeepThinkStream::new(cfg);
        let run2 = stream2.think(0xabcd_ef01, &sink2);

        assert_eq!(run1, run2, "same input must produce identical steps");

        // Verify the hash formula for step 0.
        let input_hash: u64 = 0xabcd_ef01;
        let expected_prompt = input_hash.wrapping_mul(1).rotate_left(17);
        let expected_output = expected_prompt ^ (0u64 * 0xcafe);
        assert_eq!(run1[0].prompt_hash, expected_prompt);
        assert_eq!(run1[0].output_hash, expected_output);
    }
}
