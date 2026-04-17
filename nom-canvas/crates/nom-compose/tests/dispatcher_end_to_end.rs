//! Integration: dispatcher + stubs + task queue + credentials + provider router.

use nom_compose::backend_trait::{ComposeSpec, InterruptFlag, ProgressSink};
use nom_compose::backends::register_all_stubs;
use nom_compose::credential_store::CredentialStore;
use nom_compose::dispatch::ComposeDispatcher;
use nom_compose::kind::NomKind;
use nom_compose::provider_router::{FallbackStrategy, ProviderRouter};
use nom_compose::task_queue::TaskQueue;

struct NoopProgress;
impl ProgressSink for NoopProgress {
    fn notify(&self, _p: u32, _m: &str) {}
}

fn interrupt() -> InterruptFlag {
    InterruptFlag::new()
}

#[test]
fn dispatcher_handles_all_11_kinds() {
    let mut d = ComposeDispatcher::new();
    register_all_stubs(&mut d);
    let progress = NoopProgress;
    let i = interrupt();
    for kind in [
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
    ] {
        let spec = ComposeSpec {
            kind,
            params: vec![("request_id".to_string(), "abc".to_string())],
        };
        let result = d.dispatch(&spec, &progress, &i);
        assert!(
            result.is_ok(),
            "dispatch failed for kind {:?}: {:?}",
            kind,
            result
        );
    }
}

#[test]
fn task_queue_honors_concurrency_caps() {
    let mut q = TaskQueue::new();
    q.set_cap(NomKind::MediaVideo, 2);
    let id1 = q.enqueue(NomKind::MediaVideo);
    let id2 = q.enqueue(NomKind::MediaVideo);
    let id3 = q.enqueue(NomKind::MediaVideo);
    // Start first two; third should be held back until one completes.
    let s1 = q.start_next_for(NomKind::MediaVideo);
    let s2 = q.start_next_for(NomKind::MediaVideo);
    let s3 = q.start_next_for(NomKind::MediaVideo);
    assert_eq!(s1, Some(id1));
    assert_eq!(s2, Some(id2));
    assert_eq!(s3, None, "cap=2 so third should not start yet");
    q.mark_completed(id1);
    assert_eq!(
        q.start_next_for(NomKind::MediaVideo),
        Some(id3),
        "after completing id1, id3 should start"
    );
}

#[test]
fn credential_store_redacts_params() {
    let mut store = CredentialStore::new();
    store.put("api_key", "vendor-a", b"secret-token".to_vec());
    let mut spec = ComposeSpec {
        kind: NomKind::MediaImage,
        params: vec![
            ("credential:api_key".to_string(), "my-real-key".to_string()),
            ("model".to_string(), "model-a".to_string()),
        ],
    };
    store.redact_in_spec(&mut spec);
    assert_eq!(
        spec.params
            .iter()
            .find(|(k, _)| k == "credential:api_key")
            .map(|(_, v)| v.as_str()),
        Some("<redacted>")
    );
    assert_eq!(
        spec.params
            .iter()
            .find(|(k, _)| k == "model")
            .map(|(_, v)| v.as_str()),
        Some("model-a")
    );
}

#[test]
fn provider_router_fallback_skips_full_vendor() {
    let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
    r.add_vendor("alpha", 100);
    r.add_vendor("beta", 100);
    r.add_vendor("gamma", 100);
    // Fill alpha completely.
    r.record_use("alpha", 100);
    let picked = r
        .pick_vendor(None)
        .expect("at least one vendor should remain");
    assert_ne!(picked, "alpha", "fallback must skip the full vendor");
}

#[test]
fn provider_router_empty_returns_none() {
    let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
    assert!(r.pick_vendor(None).is_none());
}
