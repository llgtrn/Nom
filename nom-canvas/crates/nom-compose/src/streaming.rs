#![deny(unsafe_code)]

use crate::glue::AiGlueOrchestrator;
use crate::context::ComposeContext;
use std::sync::Arc;

/// A token emitted during streaming generation
#[derive(Debug, Clone)]
pub struct StreamToken {
    pub text: String,
    pub is_final: bool,
}

/// Streams .nomx tokens from AiGlueOrchestrator, switchable between streaming and batch mode
pub struct SwitchableStream {
    orchestrator: Arc<AiGlueOrchestrator>,
    streaming_mode: bool,
}

impl SwitchableStream {
    pub fn new(orchestrator: Arc<AiGlueOrchestrator>) -> Self {
        Self {
            orchestrator,
            streaming_mode: true,
        }
    }

    pub fn set_streaming(&mut self, enabled: bool) {
        self.streaming_mode = enabled;
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming_mode
    }

    /// Generate tokens for a compose request.
    /// In streaming mode: emits tokens one by one via callback.
    /// In batch mode: collects all tokens, calls callback once with final.
    pub fn generate<F>(&self, ctx: &ComposeContext, mut on_token: F) -> Result<(), String>
    where
        F: FnMut(StreamToken),
    {
        let blueprint = self.orchestrator.generate_blueprint(ctx)?;

        if self.streaming_mode {
            // Simulate token-by-token streaming
            let words: Vec<&str> = blueprint.nomx_source.split_whitespace().collect();
            for (i, word) in words.iter().enumerate() {
                let is_final = i == words.len() - 1;
                on_token(StreamToken {
                    text: format!("{} ", word),
                    is_final,
                });
            }
        } else {
            on_token(StreamToken {
                text: blueprint.nomx_source.clone(),
                is_final: true,
            });
        }

        Ok(())
    }

    pub fn generate_batch(&self, ctx: &ComposeContext) -> Result<String, String> {
        let mut result = String::new();
        self.generate(ctx, |token| result.push_str(&token.text))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glue::{AiGlueOrchestrator, StubLlmFn};
    use crate::context::ComposeContext;

    fn make_stream(response: &str) -> SwitchableStream {
        let llm = StubLlmFn { response: response.to_string() };
        let orchestrator = Arc::new(AiGlueOrchestrator::new(Box::new(llm)));
        SwitchableStream::new(orchestrator)
    }

    #[test]
    fn test_switchable_stream_default_is_streaming() {
        let stream = make_stream("some output");
        assert!(stream.is_streaming(), "default mode must be streaming");
    }

    #[test]
    fn test_switchable_stream_batch_mode() {
        let mut stream = make_stream("define compose_video that renders frames");
        stream.set_streaming(false);
        let ctx = ComposeContext::new("video", "my-scene");
        let result = stream.generate_batch(&ctx).unwrap();
        assert_eq!(result, "define compose_video that renders frames");
    }

    #[test]
    fn test_switchable_stream_streaming_emits_tokens() {
        let stream = make_stream("alpha beta gamma");
        let ctx = ComposeContext::new("audio", "track-1");
        let mut tokens: Vec<StreamToken> = vec![];
        stream.generate(&ctx, |t| tokens.push(t)).unwrap();
        assert_eq!(tokens.len(), 3, "must emit one token per word");
        assert!(tokens[0].text.contains("alpha"));
        assert!(tokens[1].text.contains("beta"));
        assert!(tokens[2].text.contains("gamma"));
    }

    #[test]
    fn test_stream_token_is_final_on_last() {
        let stream = make_stream("first second third");
        let ctx = ComposeContext::new("image", "scene");
        let mut tokens: Vec<StreamToken> = vec![];
        stream.generate(&ctx, |t| tokens.push(t)).unwrap();
        assert!(!tokens[0].is_final, "first token must not be final");
        assert!(!tokens[1].is_final, "middle token must not be final");
        assert!(tokens[2].is_final, "last token must be final");
    }
}
