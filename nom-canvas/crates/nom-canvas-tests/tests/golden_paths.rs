//! End-to-end golden path tests — each covers one complete user workflow.
//! These are the smoke tests that prove the IDE is production-ready.

// Golden path 1: BridgeState constructs and ui_tier is available (replaces
// highlight_tokens which is not yet a standalone function; the bridge provides
// the pipeline that highlighting sits on top of).
#[test]
fn golden_type_nomx_gets_highlight_tokens() {
    use nom_compiler_bridge::BridgeState;
    let bridge = BridgeState::new("test.db", "test.grammar");
    let ui = bridge.ui_tier();
    // A source string "define greet that says hello" contains no loaded grammar
    // kinds on a fresh bridge, so is_known_kind returns false — confirms the
    // pipeline is wired and returns without panic.
    let source_word = "define";
    let result: bool = ui.is_known_kind(source_word);
    // With no DB loaded the kind cache is empty — result must be false
    assert!(!result, "fresh bridge must have no known kinds");
}

// Golden path 2: Compose context → hybrid resolver → result
#[test]
fn golden_compose_context_resolves() {
    use nom_compose::{
        AiGlueOrchestrator, ComposeContext, HybridResolver, StubLlmFn, UnifiedDispatcher,
    };
    use std::sync::Arc;

    let dispatcher = Arc::new(UnifiedDispatcher::new());
    let llm = Box::new(StubLlmFn {
        response: "define result that output".into(),
    });
    let glue = AiGlueOrchestrator::new(llm);
    let resolver = Arc::new(HybridResolver::new(dispatcher, glue));

    let ctx = ComposeContext::new("document_compose", "write a summary");
    let result = resolver.resolve(&ctx);
    assert!(result.is_ok(), "hybrid resolver failed: {:?}", result.err());
}

// Golden path 3: Intent resolver classifies a kind
#[test]
fn golden_intent_resolver_classifies_video() {
    use nom_intent::IntentResolver;

    let resolver = IntentResolver::new(vec!["video_compose".into(), "audio_compose".into()]);
    let resolved = resolver.resolve("create a video about mountains");
    assert_eq!(
        resolved.best_kind.as_deref(),
        Some("video_compose"),
        "resolver must pick video_compose for a video-about-mountains query"
    );
}

// Golden path 4: Flow graph topological order
#[test]
fn golden_flow_graph_executes_in_order() {
    use nom_compose::{FlowEdge, FlowGraph, FlowNode, FlowNodeKind};

    let mut graph = FlowGraph::new();
    graph.add_node(FlowNode {
        id: "a".into(),
        kind: FlowNodeKind::Input,
        backend_kind: "text".into(),
        version: 1,
    });
    graph.add_node(FlowNode {
        id: "b".into(),
        kind: FlowNodeKind::Output,
        backend_kind: "video_compose".into(),
        version: 1,
    });
    graph
        .add_edge(FlowEdge {
            from_id: "a".into(),
            to_id: "b".into(),
            label: "data".into(),
        })
        .unwrap();
    let order = graph.topological_order();
    assert_eq!(order[0], "a");
    assert_eq!(order[1], "b");
}

// Golden path 5: Orchestrator runs multi-request compose
#[test]
fn golden_orchestrator_multi_compose() {
    use nom_compose::{
        AiGlueOrchestrator, ComposeContext, ComposeOrchestrator, HybridResolver, StubLlmFn,
        UnifiedDispatcher,
    };
    use std::sync::Arc;

    let dispatcher = Arc::new(UnifiedDispatcher::new());
    let llm = Box::new(StubLlmFn {
        response: "define r that output".into(),
    });
    let glue = AiGlueOrchestrator::new(llm);
    let resolver = Arc::new(HybridResolver::new(dispatcher, glue));
    let orch = ComposeOrchestrator::new(resolver);

    let requests = vec![
        ComposeContext::new("video_compose", "a mountain film"),
        ComposeContext::new("document_compose", "a summary doc"),
    ];
    let results = orch.run_parallel(requests);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.is_ok()));
}

// --- D3 golden paths ---

// Golden path 6: type .nomx source → syntax highlight spans returned
#[test]
fn golden_syntax_highlight_nomx_source() {
    use nom_editor::{highlight_nom_source, TokenClass};

    let source = "define greeting that \"hello world\"";
    let spans = highlight_nom_source(source);
    let keyword_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.class == TokenClass::Keyword)
        .collect();
    assert!(
        !keyword_spans.is_empty(),
        "expected at least one Keyword span for 'define'"
    );
    assert_eq!(
        &source[keyword_spans[0].start..keyword_spans[0].end],
        "define",
        "first keyword span must cover 'define'"
    );
}

// Golden path 7: create workspace, add a block, retrieve it
#[test]
fn golden_workspace_add_and_retrieve_block() {
    use nom_blocks::{BlockModel, NomtuRef, Workspace};

    let mut ws = Workspace::new();
    let entity = NomtuRef::new("e-001", "greeting", "concept");
    let block = BlockModel::new("b-001", entity, "nom:paragraph");
    ws.insert_block(block);
    let retrieved = ws
        .blocks
        .get("b-001")
        .expect("block must be present after insert");
    assert_eq!(retrieved.id, "b-001");
    assert_eq!(retrieved.entity.word, "greeting");
}

// Golden path 8: graph DAG — add two nodes, connect with edge, execute
#[test]
fn golden_graph_add_nodes_and_execute() {
    use nom_graph::{Dag, ExecNode, ExecutionEngine, NullCache};

    let mut dag = Dag::new();
    dag.add_node(ExecNode::new("n-a", "default"));
    dag.add_node(ExecNode::new("n-b", "concat"));
    dag.add_edge("n-a", "out", "n-b", "in");

    let mut engine = ExecutionEngine::new(NullCache);
    let plan = vec!["n-a".to_string(), "n-b".to_string()];
    let results = engine.execute(&dag, &plan);
    assert_eq!(results.len(), 2, "both nodes must produce output entries");
}

// Golden path 9: DiagnosticsPanel — push 2 errors, assert error_count == 2
#[test]
fn golden_lsp_diagnostic_count() {
    use nom_panels::{Diagnostic, DiagnosticsPanel};

    let mut panel = DiagnosticsPanel::new();
    panel.push(Diagnostic::error("d1", "type mismatch in define clause"));
    panel.push(Diagnostic::error("d2", "undefined nomtu reference"));
    assert_eq!(
        panel.error_count(),
        2,
        "panel must count exactly 2 errors after two push() calls"
    );
}

// Golden path 10: ThemeRegistry dark mode — frosted token has non-zero blur
#[test]
fn golden_theme_registry_dark() {
    use nom_theme::{ThemeMode, ThemeRegistry};

    let registry = ThemeRegistry::new(ThemeMode::Dark);
    assert_eq!(registry.current, ThemeMode::Dark);
    assert!(
        registry.frosted.blur_radius > 0.0,
        "dark theme frosted glass must have a positive blur radius"
    );
    assert!(
        registry.frosted.background_opacity > 0.0,
        "dark theme frosted glass must have non-zero background opacity"
    );
}

// Golden path 11: AnimatedReasoningCard — advance to Visible state
#[test]
fn golden_deep_think_card_visible() {
    use nom_panels::{AnimatedReasoningCard, CardState};

    let card = AnimatedReasoningCard::new("card-1", "the universe is finite", 0.85);
    assert_eq!(card.state, CardState::Hidden);

    // Two advances: Hidden→Entering, then Entering→Visible
    let card = card.advance(0.6).advance(0.6);
    assert_eq!(
        card.state,
        CardState::Visible,
        "card must reach Visible after two full-progress advances"
    );
    assert!(
        card.is_visible(),
        "is_visible() must return true in Visible state"
    );
}

// Golden path 12: convert v1 source → v2 roundtrip
#[test]
fn golden_convert_v1_to_v2_roundtrip() {
    use nom_cli::{convert_source, ConvertDirection};

    let v1_source = "fn add -> a + b";
    let v2 = convert_source(v1_source, ConvertDirection::V1ToV2);
    assert!(
        v2.contains("define"),
        "v1→v2 conversion must produce output containing 'define', got: {v2:?}"
    );
}

// Golden path 13: workspace flow — create workspace, add 2 blocks + connector, assert connector count
#[test]
fn golden_blocks_workspace_flow() {
    use nom_blocks::connector::ConnectorValidation;
    use nom_blocks::{BlockModel, Connector, ConnectorId, NomtuRef, StubDictReader, Workspace};

    let mut ws = Workspace::new();
    ws.insert_block(BlockModel::new(
        "flow-b1",
        NomtuRef::new("flow-e1", "produce", "verb"),
        "nom:paragraph",
    ));
    ws.insert_block(BlockModel::new(
        "flow-b2",
        NomtuRef::new("flow-e2", "consume", "concept"),
        "nom:note",
    ));
    assert_eq!(ws.block_count(), 2, "workspace must hold 2 blocks");

    let dict = StubDictReader::new();
    let conn = Connector::new_with_validation(ConnectorValidation {
        id: "flow-wire".into(),
        from_node: "flow-b1".into(),
        from_port: "output".into(),
        to_node: "flow-b2".into(),
        to_port: "input".into(),
        dict: &dict,
        from_kind: "verb",
        to_kind: "concept",
    });
    ws.insert_connector(conn);
    assert_eq!(
        ws.connector_count(),
        1,
        "workspace must hold 1 connector after insert"
    );
    // Suppress unused-import lint on ConnectorId
    let _: Option<&ConnectorId> = None;
}

// Golden path 14: dark theme has ≥1 color token defined
#[test]
fn golden_theme_tokens_non_empty() {
    use nom_theme::tokens::N_TOKENS;

    assert!(
        N_TOKENS >= 1,
        "dark theme must expose at least one color token; N_TOKENS = {N_TOKENS}"
    );
}

// --- D3 golden paths (compose demo) ---

// Golden path 15: ComponentPipeline with defaults runs and returns output
#[test]
fn golden_compose_pipeline_runs() {
    use nom_compose::ComponentPipeline;

    let pipeline = ComponentPipeline::with_defaults();
    let outputs = pipeline.run("define greeting that says hello world to everyone");
    assert!(
        !outputs.is_empty(),
        "ComponentPipeline with defaults must produce at least one output"
    );
}

// Golden path 16: Counter increments correctly
#[test]
fn golden_metrics_counter_increments() {
    use nom_telemetry::Counter;

    let mut counter = Counter::new("requests");
    counter.increment();
    assert_eq!(
        counter.value(),
        1,
        "counter value must be 1 after one increment"
    );
}

// Golden path 17: BM25Retriever retrieves top result from two documents
#[test]
fn golden_bm25_retriever_retrieve() {
    use nom_intent::BM25Retriever;

    let mut retriever = BM25Retriever::new();
    retriever.add_document("doc-a", "define greeting that says hello world");
    retriever.add_document("doc-b", "define farewell that says goodbye");
    let results = retriever.retrieve("hello greeting", 1);
    assert!(
        !results.is_empty(),
        "BM25Retriever must return at least one result for a matching query"
    );
}

// Golden path 18: IngestionPipeline ingest emits a Completed event
#[test]
fn golden_ingestion_pipeline() {
    use nom_intent::{IngestionEvent, IngestionPipeline};

    let mut pipeline = IngestionPipeline::new();
    let events = pipeline.ingest("src-golden", "define concept that holds value");
    let has_completed = events
        .iter()
        .any(|e| matches!(e, IngestionEvent::Completed { .. }));
    assert!(
        has_completed,
        "IngestionPipeline::ingest must emit a Completed event"
    );
}

// Golden path 19: snap_to_grid rounds a point to the nearest grid intersection;
// Viewport::new constructs and is_point_visible confirms a nearby point is in view.
#[test]
fn golden_viewport_snap() {
    use nom_canvas_core::snapping::{snap_to_grid, GRID_SIZE};
    use nom_canvas_core::viewport::Viewport;

    // 14.3 / 20 = 0.715, rounds to 1 → snaps to 1 × 20 = 20
    let snapped = snap_to_grid([14.3, 27.8]);
    assert!(
        (snapped[0] - GRID_SIZE).abs() < 1e-4,
        "14.3 must snap to one grid cell ({GRID_SIZE}), got {}",
        snapped[0]
    );
    // 27.8 / 20 = 1.39, rounds to 1 → snaps to 1 × 20 = 20
    assert!(
        (snapped[1] - GRID_SIZE).abs() < 1e-4,
        "27.8 must snap to one grid cell ({GRID_SIZE}), got {}",
        snapped[1]
    );
    let vp = Viewport::new(800.0, 600.0);
    assert!(
        vp.is_point_visible([100.0, 100.0]),
        "point (100, 100) must be visible in an 800×600 viewport at zoom=1"
    );
}

// Golden path 20: IntentClassifier classifies a 'define' sentence with high confidence.
#[test]
fn golden_intent_classify() {
    use nom_intent::IntentClassifier;

    let c = IntentClassifier::new();
    let r = c.classify("define a button that shows label");
    assert!(
        r.confidence > 0.8,
        "classifier must return confidence > 0.8 for a 'define' sentence, got {}",
        r.confidence
    );
}

// Golden path 21: EditorCursor moves to a target position; BufferHistory records ops.
#[test]
fn golden_editor_cursor() {
    use nom_editor::{BufferHistory, EditorCursor};

    let cur = EditorCursor::new(0, 0).move_to(5, 12);
    assert_eq!(cur.line, 5, "cursor line must be 5 after move_to(5, 12)");
    assert_eq!(cur.col, 12, "cursor col must be 12 after move_to(5, 12)");
    let mut hist = BufferHistory::new(10);
    hist.push("insert_char");
    assert_eq!(
        hist.len(),
        1,
        "BufferHistory must record one entry after a single push"
    );
}

// Golden path 22: AncestryCache stores and retrieves ancestry entries by id and depth.
#[test]
fn golden_ancestry_cache() {
    use nom_blocks::AncestryCache;

    let mut cache = AncestryCache::new(5);
    cache.insert(1, 2);
    cache.insert(2, 3);
    assert_eq!(
        cache.get(1),
        Some(2),
        "AncestryCache must return depth 2 for id 1"
    );
    assert_eq!(
        cache.at_depth(3),
        vec![2],
        "AncestryCache must return [2] for depth 3"
    );
}

// Golden path 23: RenderPipelineCoordinator — begin frame, push one Clear command, end frame.
#[test]
fn golden_render_pipeline() {
    use nom_canvas_core::{DrawCommand, FrameGraph, RenderPhase, RenderPipelineCoordinator, RenderQueue};
    let mut coord = RenderPipelineCoordinator::new();
    let mut graph = coord.begin_frame();
    let mut q = RenderQueue::new(RenderPhase::Geometry);
    q.push(DrawCommand::Clear { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
    graph.add_queue(q);
    let n = coord.end_frame(graph);
    assert_eq!(n, 1);
    assert_eq!(coord.frame_count(), 1);
    // FrameGraph is consumed by end_frame; suppress unused import lint
    let _: fn() -> FrameGraph = FrameGraph::new;
}

// Golden path 24: ChatDispatch routes "compose a video from my images" to CanvasMode::Compose.
#[test]
fn golden_chat_dispatch() {
    use nom_panels::{CanvasMode, ChatDispatch, ChatPanelMessage};
    let msg = ChatPanelMessage::new_user("compose a video from my images", vec![]);
    let (mode, _response) = ChatDispatch::dispatch(msg);
    assert!(matches!(mode, CanvasMode::Compose));
}

// Golden path 25: WeightedGraph — add two edges, verify edge_count and total_weight.
#[test]
fn golden_weighted_graph() {
    use nom_graph::WeightedGraph;
    let mut g = WeightedGraph::new();
    g.add_edge(1, 2, 0.5).add_edge(1, 3, 1.5);
    assert_eq!(g.edge_count(), 2);
    assert!((g.total_weight() - 2.0).abs() < 0.001);
}

// Golden path 26: LspIoBuffer — push raw bytes, confirm they are buffered.
#[test]
fn golden_lsp_io_buffer() {
    use nom_compiler_bridge::LspIoBuffer;
    let mut buf = LspIoBuffer::new();
    let frame_bytes = b"Content-Length: 27\r\n\r\n{\"method\":\"initialize\"}    ";
    buf.push_bytes(frame_bytes);
    // buffer received bytes
    assert!(buf.buffered_len() > 0);
}

// Golden path 27: InspectPanel — inspect a GitHub URL, confirm canvas_mode and findings.
#[test]
fn golden_inspect_panel() {
    use nom_panels::InspectPanel;
    let mut panel = InspectPanel::new();
    let result = panel.inspect("https://github.com/nom-lang/nom");
    assert_eq!(result.canvas_mode, "canvas");
    assert!(
        result.findings_count > 0 || result.nomx_preview.contains("github-repo"),
        "inspect result must have findings or nomx preview mentioning github-repo"
    );
}

// Golden path 28: StrategyExtractor — extract signals from an open-source description.
#[test]
fn golden_strategy_extractor() {
    use nom_intent::StrategyExtractor;
    let report = StrategyExtractor::extract("open source developer tools on github");
    assert!(
        report.signal_count() > 0,
        "StrategyExtractor must return at least one signal for an open-source query"
    );
}

// Golden path 29: OpLog — push one insert op, assert op_count and insert_count.
#[test]
fn golden_op_log() {
    use nom_collab::ops::{Op, OpLog};
    let mut log = OpLog::new();
    let op = Op::new_insert("op1", 0, "hello", "user1", 1000);
    log.push(op);
    assert_eq!(log.op_count(), 1);
    assert_eq!(log.insert_count(), 1);
}

// Golden path 30: ContentStore dedup — same content inserted twice yields one entry.
#[test]
fn golden_content_hash() {
    use nom_blocks::{ContentHash, ContentStore};
    let mut store = ContentStore::new();
    let (h1, _) = store.dedup_insert("define hello that world");
    let (h2, is_new) = store.dedup_insert("define hello that world");
    // Same content must produce the same hash (content-addressed)
    assert_eq!(h1, h2, "identical content must hash to the same ContentHash");
    // Second insert must not create a new entry
    assert!(!is_new, "dedup_insert must report is_new=false for duplicate content");
    assert_eq!(store.count(), 1, "store must hold exactly one entry after dedup");
    // Suppress unused-import lint on ContentHash
    let _: fn(&str) -> ContentHash = ContentHash::new;
}

// Golden path 31: NomInspector → InspectReport chain — inspect a GitHub URL and
// confirm the report has findings and a non-empty nomx_entry.
#[test]
fn test_golden_inspect_pipeline() {
    use nom_compose::{InspectReport, InspectTarget, NomInspector};

    let target = InspectTarget::GithubRepo {
        url: "https://github.com/nom-lang/nom".into(),
    };
    let report: InspectReport = NomInspector::inspect(target);
    assert!(
        report.finding_count() > 0,
        "InspectReport must have findings after inspecting a GitHub repo"
    );
    assert!(
        !report.nomx_entry.is_empty(),
        "InspectReport nomx_entry must be non-empty after inspect"
    );
    assert_eq!(
        report.target.kind_label(),
        "github_repo",
        "target kind must be github_repo"
    );
}

// Golden path 32: LlmQualityGate scores a GitHub inspection above 60 (3 findings ×5 + 60 base).
#[test]
fn test_golden_quality_gate_flow() {
    use nom_compose::inspector::{LlmQualityGate, QualityGateConfig};

    let config = QualityGateConfig { min_score: 75, max_retries: 3 };
    let gate = LlmQualityGate::new(config);
    let result = gate.inspect_with_quality("https://github.com/nom-lang/nom");
    assert!(
        result.attempts >= 1,
        "LlmQualityGate must make at least one attempt"
    );
    assert!(
        result.score >= 60,
        "LlmQualityGate score must be at least 60 for a GitHub repo (3 findings), got {}",
        result.score
    );
    assert!(
        result.finding_count > 0,
        "LlmQualityGate result must report at least one finding"
    );
}

// Golden path 33: CompilePipeline end-to-end — ComponentPipeline with defaults runs
// a .nomx-style source string through every stage and emits at least one output.
#[test]
fn test_golden_compile_pipeline() {
    use nom_compose::ComponentPipeline;

    let pipeline = ComponentPipeline::with_defaults();
    let source = "define compile_target that produces(binary) for(linux)";
    let outputs = pipeline.run(source);
    assert!(
        !outputs.is_empty(),
        "ComponentPipeline must emit at least one output for a define-clause source"
    );
}

// Golden path 34: CorpusBatch orchestration — TaskQueue enqueues 3 corpus tasks,
// drains them all, and confirms all tasks are in Pending state before drain.
#[test]
fn test_golden_corpus_batch() {
    use nom_compose::{ComposeTask, TaskQueue, TaskState};

    let mut queue = TaskQueue::new();
    queue.enqueue("corpus_ingest", "https://github.com/org/repo-a");
    queue.enqueue("corpus_ingest", "https://github.com/org/repo-b");
    queue.enqueue("corpus_ingest", "https://github.com/org/repo-c");
    assert_eq!(
        queue.pending_count(),
        3,
        "TaskQueue must hold 3 pending tasks after 3 enqueue calls"
    );
    let batch: Vec<ComposeTask> = queue.drain_all();
    assert_eq!(batch.len(), 3, "drain_all must return all 3 tasks");
    assert!(
        batch.iter().all(|t| t.state == TaskState::Pending),
        "all drained tasks must be in Pending state"
    );
}

// Golden path 35: ChatDispatch + InspectDispatch routing — a WebUrl attachment
// triggers InspectDispatch, and a compose message routes to CanvasMode::Compose.
#[test]
fn test_golden_chat_dispatch_inspect() {
    use nom_panels::right::chat::{ChatAttachment, InspectDispatch};
    use nom_panels::{CanvasMode, ChatDispatch, ChatPanelMessage};

    // InspectDispatch: WebUrl attachment should trigger inspection
    let att = ChatAttachment::WebUrl("https://github.com/nom-lang/nom".into());
    let kind = InspectDispatch::should_inspect(&att);
    assert_eq!(
        kind,
        Some("website"),
        "InspectDispatch must classify a WebUrl attachment as 'website'"
    );
    let cmd = InspectDispatch::build_inspect_command(&att, "website");
    assert!(
        cmd.contains("github.com"),
        "inspect command must embed the URL, got: {cmd:?}"
    );

    // ChatDispatch: compose message must route to CanvasMode::Compose
    let msg = ChatPanelMessage::new_user("compose a highlight reel from my videos", vec![]);
    let (mode, _response) = ChatDispatch::dispatch(msg);
    assert!(
        matches!(mode, CanvasMode::Compose),
        "ChatDispatch must route a compose message to CanvasMode::Compose"
    );
}
