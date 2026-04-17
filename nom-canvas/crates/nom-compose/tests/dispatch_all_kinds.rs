//! End-to-end test: ComposeDispatcher + 11 stub backends + TaskQueue caps.

use nom_compose::backends;
use nom_compose::backend_trait::{ComposeSpec, InterruptFlag, ProgressSink};
use nom_compose::dispatch::ComposeDispatcher;
use nom_compose::kind::NomKind;
use nom_compose::task_queue::TaskQueue;

struct SilentProgress;
impl ProgressSink for SilentProgress {
    fn notify(&self, _percent: u32, _message: &str) {}
}

fn all_covered_kinds() -> Vec<NomKind> {
    vec![
        NomKind::MediaVideo,
        NomKind::MediaImage,
        NomKind::MediaAudio,
        NomKind::Media3D,
        NomKind::MediaStoryboard,
        NomKind::MediaNovelVideo,
        NomKind::ScreenWeb,
        NomKind::ScreenNative,
        NomKind::DataExtract,
        NomKind::DataQuery,
        NomKind::DataTransform,
    ]
}

#[test]
fn register_all_stubs_and_dispatch_each_kind() {
    let mut dispatcher = ComposeDispatcher::new();
    backends::register_all_stubs(&mut dispatcher);
    let interrupt = InterruptFlag::new();
    let progress = SilentProgress;
    for kind in all_covered_kinds() {
        let spec = ComposeSpec { kind, params: vec![] };
        let result = dispatcher.dispatch(&spec, &progress, &interrupt);
        assert!(result.is_ok(), "dispatch for {:?} returned error: {:?}", kind, result.err());
    }
}

#[test]
fn unregistered_kind_returns_error() {
    let dispatcher = ComposeDispatcher::new(); // no backends registered
    let interrupt = InterruptFlag::new();
    let progress = SilentProgress;
    let spec = ComposeSpec { kind: NomKind::MediaVideo, params: vec![] };
    let result = dispatcher.dispatch(&spec, &progress, &interrupt);
    assert!(result.is_err());
}

#[test]
fn task_queue_respects_video_cap() {
    let mut queue = TaskQueue::new();
    // Default cap for MediaVideo is already 2; set explicitly to document intent.
    queue.set_cap(NomKind::MediaVideo, 2);
    let a = queue.enqueue(NomKind::MediaVideo);
    let b = queue.enqueue(NomKind::MediaVideo);
    let c = queue.enqueue(NomKind::MediaVideo);
    assert_eq!(queue.running_count(NomKind::MediaVideo), 0);
    assert_eq!(queue.start_next_for(NomKind::MediaVideo), Some(a));
    assert_eq!(queue.start_next_for(NomKind::MediaVideo), Some(b));
    // Cap of 2 prevents c from starting until one of a/b completes.
    assert_eq!(queue.start_next_for(NomKind::MediaVideo), None);
    assert_eq!(queue.running_count(NomKind::MediaVideo), 2);
    queue.mark_completed(a);
    assert_eq!(queue.running_count(NomKind::MediaVideo), 1);
    assert_eq!(queue.start_next_for(NomKind::MediaVideo), Some(c));
}

#[test]
fn dispatcher_with_triggered_interrupt_still_calls_stub() {
    // Stubs don't check the interrupt flag — document that behavior.
    // Real backends must check and return ComposeError::Cancelled.
    let mut dispatcher = ComposeDispatcher::new();
    backends::register_all_stubs(&mut dispatcher);
    let interrupt = InterruptFlag::new();
    interrupt.set();
    let progress = SilentProgress;
    let spec = ComposeSpec { kind: NomKind::MediaAudio, params: vec![] };
    let result = dispatcher.dispatch(&spec, &progress, &interrupt);
    assert!(result.is_ok(), "stub backend does not check interrupt — future real impls must");
}

#[test]
fn register_all_stubs_is_idempotent_per_last_wins() {
    // Calling register_all_stubs twice should not panic; the latter
    // registration wins per dispatcher semantics.
    let mut dispatcher = ComposeDispatcher::new();
    backends::register_all_stubs(&mut dispatcher);
    backends::register_all_stubs(&mut dispatcher);
    let interrupt = InterruptFlag::new();
    let progress = SilentProgress;
    let spec = ComposeSpec { kind: NomKind::MediaImage, params: vec![] };
    assert!(dispatcher.dispatch(&spec, &progress, &interrupt).is_ok());
}
