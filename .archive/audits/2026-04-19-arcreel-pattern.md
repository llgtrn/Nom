# ArcReel Pattern Audit — 2026-04-19

**Reference repo:** `C:/Users/trngh/Documents/APP/Accelworld/services/other4/ArcReel-main/`
**Auditor:** Pattern-extraction analyst (subagent)
**Scope:** 5-phase video orchestration, storyboard generation, video agent architecture, external media API integration.
**Constraint:** Analysis only — no production code written.

---

## 1. Pattern Summary

ArcReel is an AGPL-3.0 AI video generation workspace (Novel → Short Video). It uses a **multi-agent orchestration layer** (Claude Agent SDK) over a **Python FastAPI server** with a **per-provider async generation queue** and **filesystem-backed project state**.

### 1.1 High-Level 5-Phase Production Pipeline

The README distills the flow to 5 macro phases:

```
Novel → Script → Character/Clue Design → Storyboard Images → Video Clips → Final Video (FFmpeg)
```

In practice, the `manga-workflow` skill (`.claude/skills/manga-workflow/SKILL.md`) implements **9 granular stages** (0–8) with automatic state detection and resumption:

| Stage | Trigger | Actor | Output |
|-------|---------|-------|--------|
| 0 Project Setup | `projects/{name}/` missing | Main Agent | `project.json`, directory tree |
| 1 Global Character/Clue Design | `characters` or `clues` empty in `project.json` | `analyze-characters-clues` subagent | Populated `project.json` |
| 2 Episode Splitting | `source/episode_{N}.txt` missing | Main Agent (direct) | `source/episode_{N}.txt`, `_remaining.txt` |
| 3 Per-Episode Preprocessing | `drafts/episode_N/step1_*.md` missing | `split-narration-segments` or `normalize-drama-script` subagent | `step1_segments.md` / `step1_normalized_script.md` |
| 4 JSON Script Generation | `scripts/episode_{N}.json` missing | `create-episode-script` subagent → `generate_script.py` | `scripts/episode_{N}.json` |
| 5 Character Design | Missing `character_sheet` | `generate-assets` subagent → `generate_character.py` | `characters/{name}.png` |
| 6 Clue Design | Missing `clue_sheet` for `importance=major` | `generate-assets` subagent → `generate_clue.py` | `clues/{name}.png` |
| 7 Storyboard Generation | Missing `storyboard_image` | `generate-assets` subagent → `generate_storyboard.py` | `storyboards/scene_{id}.png` |
| 8 Video Generation | Missing `video_clip` | `generate-assets` subagent → `generate_video.py` | `videos/scene_{id}.mp4` |
| Post | Compose / Export | `compose-video` skill → `compose_video.py` | `output/` final MP4, Jianying draft ZIP |

**State detection** is deterministic: the skill reads `project.json` and globs the filesystem, stopping at the first missing artifact. This allows resumption from any stage.

### 1.2 Storyboard Generation & Scene Scheduling

Storyboard generation is not a simple loop; it uses **dependency-aware batching** to preserve visual continuity.

**Key constructs:**
- `StoryboardTaskPlan` (`lib/storyboard_sequence.py:13`) — frozen dataclass encoding:
  - `resource_id` (e.g., `E1S01`)
  - `dependency_resource_id` (previous scene/segment for visual continuity)
  - `dependency_group` / `dependency_index` (for queue ordering)
- `build_storyboard_dependency_plan()` (`lib/storyboard_sequence.py:88`) — builds the dependency chain. A new group starts when:
  - `segment_break = true`
  - No previous resource
  - Previous resource not in the selected batch
- `resolve_previous_storyboard_path()` (`lib/storyboard_sequence.py:55`) — resolves the prior scene image path so the next generation can reference it.

**Reference image cascade** (storyboard prompt building):
1. `character_sheet` images for every character in the scene
2. `clue_sheet` images for every clue in the scene
3. Previous storyboard image (unless `segment_break`)
4. Optional `style` / `style_description` from `project.json`

**Content modes** drive aspect ratio and data structure:
- `narration`: `segments` array, 9:16 vertical, 4s default per segment
- `drama`: `scenes` array, 16:9 horizontal, 8s default per scene

### 1.3 Video Agent Architecture

ArcReel uses a **Orchestration Skill + Focused Subagent** pattern:

- **Main Agent** (orchestrator): never loads novel text. It detects state, dispatches subagents, shows summaries, and waits for user confirmation between stages.
- **Subagents**: single-purpose, e.g.:
  - `analyze-characters-clues`
  - `split-narration-segments`
  - `normalize-drama-script`
  - `create-episode-script`
  - `generate-assets` (used for characters, clues, storyboards, and video)
- **Skills** (deterministic scripts): `generate_storyboard.py`, `generate_video.py`, `compose_video.py`, `generate_script.py`, etc. These handle API calls, file I/O, and queue submission.

This protects context space: subagents ingest large texts internally and return only a summary to the main agent.

### 1.4 Integration with External Media APIs

ArcReel abstracts all media generation through **backend protocols** and a **unified generation queue**.

#### Backend Protocols

- `ImageBackend` (`lib/image_backends/base.py:64`) — Protocol with:
  - `generate(request: ImageGenerationRequest) -> ImageGenerationResult`
  - Capabilities: `TEXT_TO_IMAGE`, `IMAGE_TO_IMAGE`
- `VideoBackend` (`lib/video_backends/base.py:148`) — Protocol with:
  - `generate(request: VideoGenerationRequest) -> VideoGenerationResult`
  - Capabilities: `TEXT_TO_VIDEO`, `IMAGE_TO_VIDEO`, `GENERATE_AUDIO`, `NEGATIVE_PROMPT`, `VIDEO_EXTEND`, `SEED_CONTROL`, `FLEX_TIER`
- `TextBackend` — Used for overview/script generation with structured outputs (Pydantic schemas).

#### Provider Registry & Custom Providers

- `PROVIDER_REGISTRY` (`lib/config/registry.py`) — Built-in providers: **Gemini** (Google), **Ark** (ByteDance/Volcano Ark), **Grok** (xAI), **OpenAI**.
- **Custom providers**: any OpenAI-compatible or Google-compatible API. Auto-discovery via `/v1/models`. Stored in DB (`custom_provider` / `custom_provider_model` tables).
- Backend resolution priority:
  1. Payload explicit provider
  2. Project-level `video_backend` / `image_backend` (`provider/model` format)
  3. Global default from `ConfigResolver`

#### Generation Queue & Worker

- `GenerationQueue` (`lib/generation_queue.py:29`) — Singleton wrapping `TaskRepository`. Supports:
  - `enqueue_task()` with `dependency_task_id`, `dependency_group`, `dependency_index`
  - `claim_next_task(media_type)` — FIFO per media type
  - `requeue_running_tasks()` — crash recovery
  - Lease-based worker heartbeat (`TASK_WORKER_LEASE_TTL_SEC = 10.0`)
- `GenerationWorker` (`lib/generation_worker.py:195`) — Async worker with **per-provider concurrency pools**:
  - `ProviderPool` (`generation_worker.py:45`) — independent `image_max` and `video_max` lanes per provider
  - `_claim_tasks()` (`generation_worker.py:392`) — claims FIFO tasks per media type; if a provider pool is full, requeues the task and stops claiming that lane to avoid head-of-line blocking
  - `_process_task()` dispatches to `execute_generation_task()` in `server/services/generation_tasks.py`

#### Task Execution

`execute_generation_task()` (`server/services/generation_tasks.py:882`) routes by `task_type`:

| Task Type | Executor | Key Action |
|-----------|----------|------------|
| `storyboard` | `execute_storyboard_task()` | Calls `MediaGenerator.generate_image_async()` with reference images; updates `storyboard_image` in script JSON |
| `video` | `execute_video_task()` | Calls `MediaGenerator.generate_video_async()` with `start_image=storyboard_file`; updates `video_clip` in script JSON; extracts thumbnail |
| `character` | `execute_character_task()` | Generates `characters/{name}.png`; updates `project.json` |
| `clue` | `execute_clue_task()` | Generates `clues/{name}.png`; updates `project.json` |

#### MediaGenerator Middle Layer

`MediaGenerator` (`lib/media_generator.py:33`) wraps backend calls with:
- Automatic **version management** (via `VersionManager`)
- **Usage tracking** (`UsageTracker` start/finish call recording)
- **Rate limiting** (`RateLimiter`)
- Output path inference:
  - `storyboards/scene_{resource_id}.png`
  - `videos/scene_{resource_id}.mp4`
  - `characters/{resource_id}.png`
  - `clues/{resource_id}.png`

#### Version Management

`VersionManager` (`lib/version_manager.py:28`) keeps a `versions.json` log per project:
- `add_version()` — copies current file to `versions/{type}/{resource_id}_v{N}_{timestamp}.{ext}`
- `restore_version()` — copies back from version archive
- `ensure_current_tracked()` — migration helper for pre-existing files

---

## 2. Key Source Files

| File | Lines | Role |
|------|-------|------|
| `agent_runtime_profile/.claude/skills/manga-workflow/SKILL.md` | 222 | **Orchestration spec** — 9 stages, state detection rules, subagent dispatch protocol |
| `agent_runtime_profile/.claude/skills/generate-storyboard/scripts/generate_storyboard.py` | 357 | Storyboard batch submission with dependency planning and reference image collection |
| `agent_runtime_profile/.claude/skills/generate-video/scripts/generate_video.py` | 792 | Video batch submission with checkpoint/resume, episode filtering, and duration validation |
| `agent_runtime_profile/.claude/skills/compose-video/scripts/compose_video.py` | 309 | FFmpeg post-processing: concat (simple or xfade transitions), BGM mix, intro/outro |
| `lib/storyboard_sequence.py` | 135 | Dependency graph builder for storyboard continuity (`StoryboardTaskPlan`, `build_storyboard_dependency_plan`) |
| `lib/generation_queue.py` | 256 | Async task queue singleton (`GenerationQueue`) with enqueue/claim/cancel/lease primitives |
| `lib/generation_worker.py` | 522 | Per-provider pool worker (`GenerationWorker`, `ProviderPool`) with lease-based scheduling |
| `lib/media_generator.py` | 439 | Middle layer (`MediaGenerator`) bridging backends ↔ version manager ↔ usage tracker |
| `lib/project_manager.py` | 1535 | Filesystem project state manager (`ProjectManager`) — scripts, characters, clues, atomic JSON writes |
| `lib/script_models.py` | 146 | Pydantic schema for structured script output (`NarrationEpisodeScript`, `DramaEpisodeScript`) |
| `server/services/generation_tasks.py` | 904 | Task executors (`execute_storyboard_task`, `execute_video_task`, etc.) and backend resolution |
| `lib/video_backends/base.py` | 160 | `VideoBackend` Protocol, `VideoGenerationRequest`, `VideoGenerationResult`, `poll_with_retry` |
| `lib/image_backends/base.py` | 73 | `ImageBackend` Protocol, `ImageGenerationRequest`, `ImageGenerationResult` |
| `lib/version_manager.py` | 356 | `VersionManager` — asset versioning, rollback, file archive |
| `lib/config/registry.py` | — | `PROVIDER_REGISTRY` — built-in provider metadata (models, capabilities, pricing) |

---

## 3. Nom Mapping

### 3.1 Existing Nom Stubs (Current State)

Nom already has placeholder types for storyboard/video pipelines:

- `StoryboardPhase` (`nom-canvas/crates/nom-compose/src/storyboard.rs:3`)
  - Enum: `Concept`, `Script`, `VisualPlan`, `Render`, `Export`
  - `phase_index()`, `next()`, `phase_name()`
- `StoryboardPlan` / `StoryboardExecutor` (`storyboard.rs:72` / `storyboard.rs:116`)
  - Step list + advancement tracker with `progress_pct()`
- `StoryboardComposer` (`nom-canvas/crates/nom-compose/src/storyboard_compose.rs:115`)
  - Act/panel model: `StoryboardAct`, `StoryboardPanel`, `SceneType`
- `MediaPipeline` (`nom-canvas/crates/nom-compose/src/media_pipeline.rs:28`)
  - Enum `PipelineStage`: `ScriptGeneration`, `AssetCollection`, `Composition`, `Encoding`, `PostProcessing`
  - `run()` produces a synthetic MP4 or delegates to FFmpeg

**Gap:** All of the above are **stubs**. `StoryboardPhase` is explicitly listed in `ROADMAP_TO_100.md` as **NOT ADOPTED** — "`StoryboardPhase` stubbed".

### 3.2 Recommended Mapping from ArcReel → Nom

| ArcReel Concept | Nom Target | Notes |
|-----------------|------------|-------|
| `manga-workflow` 9-stage state machine | Extend `StoryboardPhase` or replace with `StoryboardPhase::{Script, VisualPlan, Render, Export}` plus sub-states | Nom's 5 phases are coarser. Could keep 5 top-level phases and add `StoryboardStep` variants for the 9 sub-stages. |
| `StoryboardTaskPlan` + `build_storyboard_dependency_plan()` | New module in `nom-compose/src/storyboard_queue.rs` | Rust-native dependency plan builder; replace `dependency_group` string with a strongly-typed `SceneGroup` id. |
| `GenerationQueue` + `TaskRepository` | Reuse `nom-compose` async runtime or `nom-canvas` task system | ArcReel uses SQLAlchemy + SQLite/PostgreSQL. Nom would likely use an in-memory queue or embed `rusqlite` / `sled`. |
| `GenerationWorker` + `ProviderPool` | New `MediaWorker` struct in `media_pipeline.rs` | Per-provider concurrency limits (`image_max`, `video_max`) map naturally to Tokio `Semaphore` per provider. |
| `ImageBackend` / `VideoBackend` Protocols | Rust traits in `nom-compose/src/backends/` | Nom already has `backends/mod.rs` and `backends/storyboard.rs` — extend these into full `ImageBackend` and `VideoBackend` traits. |
| `MediaGenerator` | New `MediaGenerator` struct in `media_pipeline.rs` | Wrap backend calls with version management + usage tracking. |
| `ProjectManager` | `nom-blocks` workspace / `nom-compose` context | Nom's block tree / `ComposeContext` already holds project state. Need to add `generated_assets` tracking per scene/segment. |
| `script_models.py` (`NarrationEpisodeScript`, `DramaEpisodeScript`) | Serde structs in `nom-compose/src/script_models.rs` | Define `Scene`, `Segment`, `ImagePrompt`, `VideoPrompt`, `GeneratedAssets` with JSON schema support. |
| `VersionManager` | New `VersionManager` in `nom-compose/src/version_manager.rs` | Filesystem-based version archive; simpler than ArcReel because Nom may not need cross-project rollback UI. |
| `compose_video.py` (FFmpeg) | Extend `MediaPipeline::run()` in `media_pipeline.rs` | Already shells out to FFmpeg; add xfade transition filter chains and BGM mixing. |
| `content_mode` (`narration` / `drama`) | Enum in script models | Drives aspect ratio, default duration, and audio pipeline (narration = no AI audio, drama = AI dialogue). |

### 3.3 Critical Design Decisions to Port

1. **Reference Image Cascade** — When generating a storyboard, pass `character_sheet` + `clue_sheet` + previous storyboard as `ReferenceImage` list. Nom's backend trait must accept a `Vec<ReferenceImage>`.
2. **Checkpoint / Resume** — `generate_video.py` writes `.checkpoint_ep{N}.json` with `completed_scenes`. Nom should persist checkpoint state in the block tree or a sidecar JSON file.
3. **Per-Provider Concurrency** — Do not use a single global semaphore. Use one `Semaphore` per `(provider, media_type)` pair, exactly like `ProviderPool`.
4. **Structured Prompts** — ArcReel uses Pydantic `ImagePrompt` / `VideoPrompt` dicts that get serialized to YAML for some providers. Nom should keep structured prompt types and normalize them per backend.
5. **Negative Prompt for BGM Exclusion** — ArcReel passes `negative_prompt="background music, BGM, soundtrack, musical accompaniment"` to video backends. Nom should expose `negative_prompt` on `VideoGenerationRequest`.
6. **Atomic Project Writes** — `ProjectManager._atomic_write_json()` uses temp-file + `os.replace` + `fcntl` file locking. Nom should use `tokio::fs::write` to a temp path + `rename` for atomicity.

---

## 4. Licensing / Complexity Notes

### License
- **AGPL-3.0** (`ArcReel-main/LICENSE`).
- **Implication for Nom:** Nom cannot copy-paste ArcReel source into its own codebase without triggering AGPL-3.0 obligations (source publication for network use). However, **reading and re-implementing the architecture in Rust is clean-room compatible** because we are analyzing patterns, not translating code. All class/function names cited here are for attribution of ideas, not for verbatim reuse.

### Complexity Metrics

| Dimension | ArcReel Scale | Nom Implication |
|-----------|---------------|-----------------|
| Total Python LOC | ~10,000+ (289 `.py` files) | Full port is a large feature, not a refactor. |
| Agent Skills | 9 skills + 7 subagent definitions | Nom does not use Claude Agent SDK; equivalent orchestration must be built into `nom-compose` or a Nomx runtime. |
| Backend Providers | 4 built-in + custom provider discovery | High surface area. Nom should start with 1–2 providers (e.g., Gemini + OpenAI) and add custom provider support later. |
| DB Schema | 9 Alembic migrations (tasks, usage, credentials, custom providers) | Nom can defer DB persistence initially; in-memory queue + filesystem versions is enough for MVP. |
| FFmpeg Integration | Concat, xfade, BGM mix, thumbnail extraction | Nom already shells out to FFmpeg in `media_pipeline.rs`; extend filter_complex building. |
| Web UI | React 19 + FastAPI SSE streams | Nom is not a web app; skip SSE streaming, REST routers, and JWT auth. |

### What to Skip
- FastAPI server, auth, JWT, API keys, user management
- React frontend, SSE event streaming
- Cost estimation, multi-currency usage tracking, Jianying export
- Custom provider discovery (`/v1/models` auto-registration)
- Full Alembic/SQLAlchemy ORM layer

### What to Keep
- 5-phase (or 9-substage) state machine
- Dependency-aware storyboard batching
- Per-provider concurrency pools
- Backend trait abstraction
- Version manager for asset rollback
- Checkpoint/resume for long generation jobs
- Content mode switching (narration vs drama)
- Reference image cascade for consistency

---

## 5. Adoption Effort Estimate

### Option A: Minimal Adoption (Wire the Stubs)
**Goal:** Make `StoryboardPhase` and `MediaPipeline` functional enough to generate a video from a Nomx script.

| Task | Effort | Owner |
|------|--------|-------|
| Define `Script`, `Scene`, `Segment`, `GeneratedAssets` serde structs | 1–2 days | nom-compose |
| Implement `StoryboardTaskPlan` + dependency builder in Rust | 2–3 days | nom-compose |
| Add `ImageBackend` and `VideoBackend` traits with 1 provider (Gemini) | 3–4 days | nom-compose |
| Port `MediaGenerator` middle layer (version + usage tracking) | 2–3 days | nom-compose |
| Add per-provider `ProviderPool` / `MediaWorker` queue | 3–4 days | nom-compose |
| Extend `MediaPipeline::run()` to accept a real script and call backends | 2–3 days | nom-compose |
| Add checkpoint/resume for video generation | 1–2 days | nom-compose |
| FFmpeg composition: concat + simple transitions | 1–2 days | nom-compose |
| **Total** | **15–23 days** | |

### Option B: Full ArcReel Parity
**Goal:** Full novel-to-video pipeline with character/clue design, episode splitting, and version rollback.

| Task | Effort | Owner |
|------|--------|-------|
| All of Option A | 15–23 days | nom-compose |
| Character / clue design agents + asset generation | 5–7 days | nom-compose |
| Episode splitting + preprocessing sub-pipeline | 4–5 days | nom-compose |
| Script generation agent (structured output from LLM) | 4–5 days | nom-compose |
| Version manager with rollback UI hooks | 3–4 days | nom-compose |
| Multi-provider backend registry (4+ providers) | 5–7 days | nom-compose |
| Custom provider support | 4–5 days | nom-compose |
| **Total** | **40–56 days** | |

### Recommended Path

Nom should pursue **Option A first**:
1. Replace the stubbed `MediaPipeline::run()` with a real 5-stage executor.
2. Introduce `StoryboardTaskPlan` and dependency batching so storyboards maintain visual continuity.
3. Add one `VideoBackend` (Gemini Veo via `google-genai` REST API) and one `ImageBackend` (Gemini Imagen).
4. Add a lightweight in-memory generation queue with per-provider `tokio::sync::Semaphore` pools.
5. Wire `StoryboardPhase` transitions to actual stage execution.

This gives Nom a working video pipeline in ~3 weeks without pulling in the full AGPL surface area of ArcReel.

---

## Appendix: ArcReel Class/Function Index (Cited)

- `StoryboardTaskPlan` — `lib/storyboard_sequence.py:13`
- `build_storyboard_dependency_plan()` — `lib/storyboard_sequence.py:88`
- `resolve_previous_storyboard_path()` — `lib/storyboard_sequence.py:55`
- `GenerationQueue` — `lib/generation_queue.py:29`
- `GenerationWorker` — `lib/generation_worker.py:195`
- `ProviderPool` — `lib/generation_worker.py:45`
- `_claim_tasks()` — `lib/generation_worker.py:392`
- `MediaGenerator` — `lib/media_generator.py:33`
- `generate_image_async()` — `lib/media_generator.py:160`
- `generate_video_async()` — `lib/media_generator.py:316`
- `ProjectManager` — `lib/project_manager.py:42`
- `_atomic_write_json()` — `lib/project_manager.py:916`
- `update_scene_asset()` — `lib/project_manager.py:747`
- `VersionManager` — `lib/version_manager.py:28`
- `add_version()` — `lib/version_manager.py:129`
- `execute_generation_task()` — `server/services/generation_tasks.py:882`
- `execute_storyboard_task()` — `server/services/generation_tasks.py:571`
- `execute_video_task()` — `server/services/generation_tasks.py:645`
- `ImageBackend` — `lib/image_backends/base.py:64`
- `VideoBackend` — `lib/video_backends/base.py:148`
- `VideoGenerationRequest` — `lib/video_backends/base.py:109`
- `poll_with_retry()` — `lib/video_backends/base.py:30`
- `NarrationEpisodeScript` — `lib/script_models.py:106`
- `DramaEpisodeScript` — `lib/script_models.py:137`
- `GeneratedAssets` — `lib/script_models.py:70`
- `FailureRecorder` — `agent_runtime_profile/.../generate_storyboard.py:42`
- `generate_storyboard_direct()` — `agent_runtime_profile/.../generate_storyboard.py:240`
- `generate_episode_video()` — `agent_runtime_profile/.../generate_video.py:379`
- `concatenate_with_transitions()` — `agent_runtime_profile/.../compose_video.py:77`
- `add_background_music()` — `agent_runtime_profile/.../compose_video.py:168`