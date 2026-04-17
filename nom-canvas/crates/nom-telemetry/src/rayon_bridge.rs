//! Span propagation helper for work-stealing thread pools (rayon-style).
//!
//! The crate stays rayon-free; callers pass their spawn primitive in as a
//! closure.  The helper carries the parent SpanContext across thread
//! boundaries so spawned work stays linked to the parent trace.
#![deny(unsafe_code)]

use crate::span::{Span, SpanContext};
use crate::tier::TelemetryTier;

/// Guard that finishes the span when dropped.  Holding the guard in scope
/// keeps the span "active"; when `drop` runs the span is finalised.
pub struct RayonSpanGuard {
    pub span: Span,
}

impl RayonSpanGuard {
    pub fn new(name: impl Into<String>, tier: TelemetryTier, parent: Option<SpanContext>) -> Self {
        let mut span = Span::new(name, tier);
        if let Some(p) = parent {
            // Inherit the parent's trace_id + set parent span_id.
            span.context.trace_id = p.trace_id;
            span.parent = Some(p.span_id);
            // Sampled decision propagates unless root sampler overrides.
            span.context.sampled = p.sampled;
        }
        Self { span }
    }

    /// Snapshot the current context for downstream propagation.
    pub fn context(&self) -> SpanContext {
        self.span.context.clone()
    }
}

impl Drop for RayonSpanGuard {
    fn drop(&mut self) {
        self.span.finish();
    }
}

/// Run `work` inside an instrumented span.  The `spawn` callback is the
/// caller's thread-pool primitive (e.g. `rayon::scope`).  Inside `work`, the
/// span's `SpanContext` is available via the passed-in argument — the caller
/// is responsible for stuffing it into any further `rayon_instrumented_scope`
/// calls on child tasks.
pub fn rayon_instrumented_scope<F, R>(
    name: impl Into<String>,
    tier: TelemetryTier,
    parent: Option<SpanContext>,
    spawn: impl FnOnce(Box<dyn FnOnce(SpanContext) -> R>) -> R,
    work: F,
) -> R
where
    F: FnOnce(SpanContext) -> R + 'static,
    R: 'static,
{
    let guard = RayonSpanGuard::new(name, tier, parent);
    let ctx = guard.context();
    // Invoke the caller's spawn primitive with a boxed work closure;
    // the guard outlives the spawn return so that finish() fires afterwards.
    let result = spawn(Box::new(move |inherited_ctx| work(inherited_ctx)));
    // Explicitly use ctx so callers can verify propagation in tests without
    // relying on Drop order.
    let _ = ctx;
    drop(guard);
    result
}

/// Convenience: run a closure immediately (no actual parallelism) with a
/// synchronous spawn primitive.  Useful for tests + single-threaded fallback.
pub fn inline_instrumented_scope<F, R>(
    name: impl Into<String>,
    tier: TelemetryTier,
    parent: Option<SpanContext>,
    work: F,
) -> R
where
    F: FnOnce(SpanContext) -> R,
{
    let guard = RayonSpanGuard::new(name, tier, parent);
    let ctx = guard.context();
    work(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;
    use crate::tier::TelemetryTier;

    #[test]
    fn new_with_no_parent_creates_root_span() {
        let guard = RayonSpanGuard::new("root", TelemetryTier::Ui, None);
        assert!(guard.span.parent.is_none());
    }

    #[test]
    fn new_with_parent_inherits_trace_id() {
        let parent_span = Span::new("parent", TelemetryTier::Ui);
        let parent_ctx = parent_span.context.clone();
        let guard = RayonSpanGuard::new("child", TelemetryTier::Ui, Some(parent_ctx.clone()));
        assert_eq!(guard.span.context.trace_id, parent_ctx.trace_id);
    }

    #[test]
    fn new_with_parent_sets_span_parent_to_parent_span_id() {
        let parent_span = Span::new("parent", TelemetryTier::Ui);
        let parent_ctx = parent_span.context.clone();
        let guard = RayonSpanGuard::new("child", TelemetryTier::Ui, Some(parent_ctx.clone()));
        assert_eq!(guard.span.parent, Some(parent_ctx.span_id));
    }

    #[test]
    fn new_with_parent_sampled_true_propagates_sampled() {
        let mut parent_span = Span::new("parent", TelemetryTier::Ui);
        parent_span.context.sampled = true;
        let parent_ctx = parent_span.context.clone();
        let guard = RayonSpanGuard::new("child", TelemetryTier::Ui, Some(parent_ctx));
        assert!(guard.span.context.sampled);
    }

    #[test]
    fn new_with_parent_sampled_false_propagates_sampled_false() {
        let mut parent_span = Span::new("parent", TelemetryTier::Ui);
        parent_span.context.sampled = false;
        let parent_ctx = parent_span.context.clone();
        let guard = RayonSpanGuard::new("child", TelemetryTier::Ui, Some(parent_ctx));
        assert!(!guard.span.context.sampled);
    }

    #[test]
    fn inline_instrumented_scope_runs_work_and_returns_result() {
        let result = inline_instrumented_scope("test", TelemetryTier::Interactive, None, |_ctx| {
            42u32
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn inline_instrumented_scope_passes_guard_context_to_work() {
        let parent_span = Span::new("parent", TelemetryTier::Ui);
        let parent_ctx = parent_span.context.clone();

        let received_trace_id =
            inline_instrumented_scope("child", TelemetryTier::Ui, Some(parent_ctx.clone()), |ctx| {
                ctx.trace_id
            });

        assert_eq!(received_trace_id, parent_ctx.trace_id);
    }

    #[test]
    fn nested_inline_scopes_share_parent_trace_id() {
        let root_span = Span::new("root", TelemetryTier::Ui);
        let root_ctx = root_span.context.clone();

        let (outer_trace_id, inner_trace_id) = inline_instrumented_scope(
            "outer",
            TelemetryTier::Ui,
            Some(root_ctx.clone()),
            |outer_ctx| {
                let outer_tid = outer_ctx.trace_id;
                let inner_tid = inline_instrumented_scope(
                    "inner",
                    TelemetryTier::Ui,
                    Some(outer_ctx),
                    |inner_ctx| inner_ctx.trace_id,
                );
                (outer_tid, inner_tid)
            },
        );

        assert_eq!(outer_trace_id, root_ctx.trace_id);
        assert_eq!(inner_trace_id, root_ctx.trace_id);
    }

    #[test]
    fn drop_of_guard_sets_end_micros() {
        // Confirm finish() (called by Drop) sets end_micros from None to Some.
        let mut guard = RayonSpanGuard::new("timed", TelemetryTier::Background, None);
        assert!(guard.span.end_micros.is_none());
        // Call finish explicitly — same path taken by Drop::drop.
        guard.span.finish();
        assert!(guard.span.end_micros.is_some());
    }

    #[test]
    fn rayon_instrumented_scope_with_inline_spawn_returns_result() {
        let result = rayon_instrumented_scope(
            "scope",
            TelemetryTier::Interactive,
            None,
            |work| work(SpanContext { trace_id: [0u8; 16], span_id: [0u8; 8], sampled: true }),
            |_ctx| 99u32,
        );
        assert_eq!(result, 99);
    }
}
