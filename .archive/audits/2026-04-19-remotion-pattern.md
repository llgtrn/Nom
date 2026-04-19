# Remotion Pattern Audit — 2026-04-19

**Reference repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\remotion-main\`  
**Focus areas:** Renderer frame capture, FFmpeg integration, Lambda/serverless orchestration, Composition/timeline abstraction.  
**Auditor:** Pattern-extraction analyst (read-only, no production code written).

---

## 1. Pattern Summary

Remotion is a **React → Headless Chromium → FFmpeg** video pipeline. The high-level flow is:

1. **Bundle** a React app that exports `<Composition>` components.
2. **Open** the bundle in headless Chromium (Puppeteer) via `internalOpenBrowser()`.
3. **Seek** to each target frame by evaluating `window.remotion_setFrame(frame, compId, attempt)` in the page.
4. **Capture** the frame with CDP `Page.captureScreenshot` (`screenshotTask()`), returning a `Buffer`.
5. **Pipe or save** the buffer:
   - **Disk mode:** write to `outputDir` as `element-%09d.jpeg` / `.png`, then run FFmpeg `image2` sequence (`stitchFramesToVideo`).
   - **Parallel encoding mode:** write buffer directly to a pre-spawned FFmpeg process’s `stdin` via `image2pipe` (`prespawnFfmpeg`).
6. **Collect audio assets** per frame (`collectAssets`), download/map them, preprocess each track (`preprocessAudioTrack`), merge (`mergeAudioTrack`), compress (`compressAudio`), then mux with video.
7. **Distributed mode (Lambda):** split the frame range into chunks (`planFrameRanges`), invoke serverless renderer functions per chunk, stream progress back, then concatenate chunk files (`concatVideos` / `combineChunks`).

### Critical Data Structures

- `VideoConfig` (from `remotion/no-react`): `{ id, width, height, fps, durationInFrames, props, defaultProps, defaultCodec, defaultPixelFormat, defaultProResProfile, defaultSampleRate }` — the canonical composition descriptor.
- `FrameAndAssets` (from `render-frames.ts`): `{ frame, audioAndVideoAssets, artifactAssets, inlineAudioAssets }` — per-frame metadata collected from the browser.
- `RenderMediaProgress` (from `render-media.ts`): `{ renderedFrames, encodedFrames, encodedDoneIn, renderedDoneIn, renderEstimatedTime, progress, stitchStage }` — unified progress for local renders.
- `OverallRenderProgress` (from `serverless/src/overall-render-progress.ts`): tracks per-chunk `framesRendered`, `framesEncoded`, `lambdaInvoked`, `chunks`, `errors`, and uploads a JSON blob to S3 for polling.

---

## 2. Key Source Files

### 2.1 Renderer — Frame Capture & Browser Control

| File | Key Symbol | Responsibility |
|------|-----------|----------------|
| `packages/renderer/src/render-frames.ts` | `innerRenderFrames()`, `Pool` | Orchestrates concurrent frame rendering over a pool of Puppeteer pages. Uses `renderFrameAndRetryTargetClose()` per worker. |
| `packages/renderer/src/render-media.ts` | `internalRenderMediaRaw()`, `RenderMediaProgress` | High-level API: decides parallel encoding, calls `renderFrames` + `stitchFramesToVideo`, tracks `SlowFrame[]`. |
| `packages/renderer/src/render-frame-with-option-to-reject.ts` | `renderFrameWithOptionToReject()` | Per-frame work: `seekToFrame()` → `takeFrame()` + `collectAssets()` → asset compression/download kickoff. |
| `packages/renderer/src/take-frame.ts` | `takeFrame()` | Sets background color (transparent for png/webp, black for jpeg) then calls `screenshot()`. |
| `packages/renderer/src/screenshot-task.ts` | `screenshotTask()` | Sends CDP `Target.activateTarget`, optionally `Emulation.setDefaultBackgroundColorOverride`, then `Page.captureScreenshot` with `clip: {x, y, width, height, scale: 1}`, `captureBeyondViewport: true`, `optimizeForSpeed: true`, `fromSurface`. Returns `Buffer.from(result.data, 'base64')`. |
| `packages/renderer/src/seek-to-frame.ts` | `seekToFrame()`, `waitForReady()` | Evaluates `window.remotion_setFrame` and waits for `window.remotion_renderReady === true` via `page.mainFrame()._mainWorld.waitForFunction()`. Also handles `delayRender()` timeout introspection. |
| `packages/renderer/src/make-page.ts` | `makePage()` | Creates a new Puppeteer page, sets viewport (`width`, `height`, `deviceScaleFactor: scale`), injects bundle via `setPropsAndEnv()`, and evaluates `window.remotion_setBundleMode({type: 'composition', ...})`. |
| `packages/renderer/src/pool.ts` | `Pool` | Simple semaphore: `acquire()` pops a `Page` or queues a waiter; `release()` resolves the next waiter or pushes the page back. |

### 2.2 Renderer — FFmpeg Integration

| File | Key Symbol | Responsibility |
|------|-----------|----------------|
| `packages/renderer/src/stitch-frames-to-video.ts` | `innerStitchFramesToVideo()` | Builds FFmpeg args: `-r fps -f image2 -s WxH -start_number N -i sequence.%09d.ext`, optional audio `-i audio -c:a copy`, then `generateFfmpegArgs()` + codec-specific flags. Spawns FFmpeg via `callFfNative()`. Parses stderr with `parseFfmpegProgress()`. |
| `packages/renderer/src/prespawn-ffmpeg.ts` | `prespawnFfmpeg()` | Spawns FFmpeg early with `image2pipe` input: `-f image2pipe -s WxH -vcodec mjpeg/png -i -`. Returns `{task, getLogs, getExitStatus}`. Frames are later written to `task.stdin` in order. |
| `packages/renderer/src/call-ffmpeg.ts` | `callFf()`, `callFfNative()` | Wraps `execa()` and `spawn()` respectively. Resolves the FFmpeg binary path via `getExecutablePath()`, sets cwd/env, and wires `cancelSignal` to `task.kill()`. |
| `packages/renderer/src/ffmpeg-args.ts` | `generateFfmpegArgs()` | Returns a `string[][]` of codec args: `-c:v libx264/libvpx-vp9/libaom-av1/prores_ks`, `-pix_fmt`, `-preset`, `-crf`, `-b:v`, `-maxrate`, `-bufsize`, color-space filters (`zscale=matrix=709:matrixin=709:range=limited`), hardware accel flags, `-movflags faststart` for h264, etc. |
| `packages/renderer/src/parse-ffmpeg-progress.ts` | `parseFfmpegProgress()` | Regex-parses `frame=  123 ` or `time=00:00:01.23` from FFmpeg stderr and converts to frame index. |
| `packages/renderer/src/ensure-frames-in-order.ts` | `ensureFramesInOrder()` | Returns `{waitForRightTimeOfFrameToBeInserted, setFrameToStitch, waitForFinish}`. Ensures out-of-order rendered frames are sequenced correctly before being piped into FFmpeg stdin. |
| `packages/renderer/src/create-audio.ts` | `createAudio()` | Three-phase audio pipeline: `convertAssetsToFileUrls()` → `calculateAssetPositions()` → parallel `preprocessAudioTrack()` → `mergeAudioTrack()` → `compressAudio()`. |
| `packages/renderer/src/combine-chunks.ts` | `internalCombineChunks()` | Combines distributed video/audio chunks. Decides seamless vs. normal concat via `canConcatVideoSeamlessly()` / `canConcatAudioSeamlessly()`, then `combineVideoStreams()` / `createCombinedAudio()` / `muxVideoAndAudio()`. |

### 2.3 Core — Composition / Timeline Abstraction

| File | Key Symbol | Responsibility |
|------|-----------|----------------|
| `packages/core/src/Composition.tsx` | `Composition`, `CalcMetadataReturnType`, `CalculateMetadataFunction` | Registers a renderable scene into `CompositionManager` with `durationInFrames`, `fps`, `width`, `height`. Supports lazy component loading and `calculateMetadata()` for dynamic sizing. |
| `packages/core/src/Sequence.tsx` | `Sequence`, `SequenceProps` | Time-shifts children: `from` (start frame), `durationInFrames`. Uses React context (`SequenceContext`) to accumulate `cumulatedFrom` so nested sequences compose. Content is `null` when `absoluteFrame < cumulatedFrom + from` or past `endThreshold`. |
| `packages/core/src/CompositionManager.tsx` | `CompositionManager`, `TComposition` | Central registry of all compositions discovered in the React tree. |
| `packages/renderer/src/get-compositions.ts` | `innerGetCompositions()` | Opens bundle in browser, evaluates `window.getStaticCompositions()`, returns `VideoConfig[]`. |

### 2.4 Lambda / Serverless — Distributed Rendering & Progress

| File | Key Symbol | Responsibility |
|------|-----------|----------------|
| `packages/lambda/src/functions/index.ts` | `handler`, `routine` | AWS Lambda entrypoint. Wraps `innerHandler` with `streamifyResponse`. |
| `packages/serverless/src/inner-routine.ts` | `innerHandler()` | Router for `ServerlessRoutines`: `start`, `launch`, `renderer`, `status`, `still`, `compositions`, `info`. |
| `packages/serverless/src/plan-frame-ranges.ts` | `planFrameRanges()` | Splits a `[start, end]` frame range into chunk tuples based on `framesPerFunction`. |
| `packages/serverless/src/stream-renderer.ts` | `streamRenderer()`, `streamRendererFunctionWithRetry()` | Invokes a renderer Lambda with streaming payload. Handles messages: `lambda-invoked`, `frames-rendered`, `video-chunk-rendered`, `audio-chunk-rendered`, `chunk-complete`, `artifact-emitted`, `error-occurred`. Retries on network errors (`ETIMEDOUT`, `ECONNRESET`). |
| `packages/serverless/src/overall-render-progress.ts` | `makeOverallRenderProgress()`, `OverallProgressHelper` | Mutable progress accumulator per render job. Tracks `framesRendered[]`, `framesEncoded[]`, `lambdasInvoked[]` per chunk. Uploads JSON to object storage (S3) on every mutation with a 250ms debounce. |
| `packages/serverless/src/merge-chunks.ts` | `mergeChunksAndFinishRender()` | After all chunks complete, calls `concatVideos()` to stitch, then uploads final output to bucket and constructs `PostRenderData`. |
| `packages/lambda/src/cli/commands/render/render.ts` | `renderCommand()` | CLI that invokes `internalRenderMediaOnLambdaRaw()`, then polls `getRenderProgress()` in a `while(true)` loop with 500ms `sleep()`. |
| `packages/lambda/src/cli/commands/render/progress.ts` | `makeProgressString()` | Builds a multi-line progress bar: evaluation → lambda invoke → frame render → frame encode → chunk combine → download. |

---

## 3. Nom Mapping

### 3.1 Current State in Nom (`nom-compose/src/video_encode.rs`)

Nom currently defines minimal stubs:

- `VideoCodec` enum (`H264`, `H265`, `Vp9`, `Av1`) with `codec_name()`.
- `VideoFrame` struct (`width`, `height`, `frame_index`, `pixel_count`).
- `VideoEncoder` struct (`codec`, `fps`, `encoded_frames`) with `encode_frame()` (no-op) and `estimated_output_mb()`.
- `GpuVideoEncoder` struct wrapping `VideoEncoder` with `parallel_streams`.

**Gaps identified:**
1. No pixel buffer / GPU render target → image extraction path.
2. No FFmpeg process management or argument generation.
3. No frame-ordering gate before encode.
4. No audio pipeline (assets, mixing, compression).
5. No distributed chunking or progress tracking.
6. No composition/timeline scene graph equivalent.

### 3.2 What to Port / Reimplement

| Remotion Pattern | Nom Target | Notes |
|------------------|-----------|-------|
| `screenshotTask()` + `Page.captureScreenshot` | `nom-canvas` wgpu render target → `image::DynamicImage` → bytes | Nom must implement its own GPU→frame capture because Remotion’s Chromium path is irrelevant. Study the `clip`, `scale`, `omitBackground` semantics, not the CDP call. |
| `prespawnFfmpeg()` + `image2pipe` | New module in `nom-compose` or `nom-canvas` | Spawn FFmpeg with `-f image2pipe -vcodec mjpeg/png -i -`. Write `Vec<u8>` frames to stdin. This is the **highest-value** pattern to extract. |
| `ensureFramesInOrder()` | Frame sequencer / ordering queue | A small async primitive that holds back out-of-order frames until the next expected `frame_index` is available. Essential for parallel rendering + single FFmpeg stdin. |
| `generateFfmpegArgs()` | FFmpeg argument builder in Rust | Port the logic: codec name mapping, CRF/bitrate, pixel format (`-pix_fmt`), color space (`-colorspace:v`, `-color_primaries:v`, `-color_trc:v`, `-vf zscale=...`), x264 preset, `-movflags faststart`, hardware accel flags. |
| `parseFfmpegProgress()` | FFmpeg stderr parser | Regex on `frame=\s*(\d+)` or `time=HH:MM:SS.mm` to drive progress callbacks. |
| `createAudio()` chain | `nom-compose/src/audio.rs` (new) | Asset timing (`calculateAssetPositions`), per-track preprocessing (resample/trim), merge (mixdown), compress (AAC/Opus). Can be simplified if Nom targets a single audio input per scene. |
| `planFrameRanges()` + `streamRenderer()` + `combineChunks()` | Distributed render orchestrator (future) | If Nom ever needs cloud rendering, the chunk-splitting + retry + concat pattern is the reference. For local GPU parallel encode, the simpler `GpuVideoEncoder::encode_batch()` stub can be expanded with frame-range shards and a final concat step. |
| `Composition` + `Sequence` | Nomx scene graph / block timeline | Remotion’s timeline is React component trees with context-based time shifting. Nom needs an equivalent in its block/Nomx model: a `Scene` block with `fps`, `duration`, and child blocks that declare `from_frame` / `to_frame`. The `seekToFrame` equivalent would invoke block render with a `frame_index` parameter. |
| `makeOverallRenderProgress()` | Render job state machine | A struct tracking `frames_rendered`, `frames_encoded`, `chunks_completed`, `errors`, with a dirty-flag upload loop. Can be adapted for Nom’s CLI or Ruflo task progress. |

---

## 4. Licensing / Complexity Notes

### 4.1 License

Remotion uses a **custom two-tier license** (`LICENSE.md`), **not** MIT/Apache/BSD:

- **Free License:** allowed for individuals, non-profits, and for-profits with ≤3 employees. Explicitly allows modification for custom use cases and contributing back.
- **Disallowed:** *"It is not allowed to copy or modify Remotion code for the purpose of selling, renting, licensing, relicensing, or sublicensing your own derivate of Remotion."*
- **Company License:** required for larger for-profits; purchased via remotion.pro.

**Implication for Nom:** We cannot copy-paste Remotion source into Nom. This audit is read-only pattern extraction. Any reimplementation must be clean-room inspired by the architecture, not a direct port of the TypeScript logic.

### 4.2 Complexity & Dependencies

- **Renderer:** ~150 TypeScript source files. Heavy dependency on Node.js, `execa`, Puppeteer/Chrome DevTools Protocol, and bundled FFmpeg binaries.
- **Core / React integration:** Deeply tied to React component lifecycle and context. Not portable to Rust.
- **Lambda / Serverless:** Tightly coupled to AWS Lambda response streaming, S3, and IAM. The **orchestration pattern** (chunking → parallel workers → concat) is generic; the AWS-specific implementation is not.
- **FFmpeg knowledge encoded in Remotion:** The most reusable asset is the FFmpeg argument matrix (`generateFfmpegArgs`, `getCodecName`, color-space handling, seamless concatenation rules). This is operational knowledge, not copyrightable expression, and can be reimplemented in Rust safely.

---

## 5. Adoption Effort Estimate

| Capability | Effort | Rationale |
|------------|--------|-----------|
| **GPU → image buffer extraction** | *Already in progress* | Nom has wgpu/`nom-canvas`. Need render-to-texture → PNG/JPEG encoder (e.g., `image` crate). |
| **FFmpeg `image2pipe` encoder** | **Low** (~2–3 days) | Reimplement `prespawnFfmpeg` + `ensureFramesInOrder` in Rust: spawn `std::process::Command`, write frames to `stdin`, handle cancel/kill. |
| **FFmpeg arg builder** | **Low** (~2 days) | Port `generateFfmpegArgs` logic into a Rust function returning `Vec<String>`. |
| **Progress parsing & callbacks** | **Low** (~1 day) | Port `parseFfmpegProgress` regex + `RenderMediaProgress` struct. |
| **Audio pipeline (assets → mix → compress)** | **Medium** (~1–2 weeks) | If Nom needs multi-track audio: port `createAudio` chain. If single-track, reduce to FFmpeg `-i audio.mp3 -c:a copy`. |
| **Scene graph / timeline blocks** | **Medium-High** (~2–3 weeks) | Design Nomx `Scene`/`Clip` blocks with `from`/`duration`. Implement frame-seeking evaluation. This is architectural design work, not a port. |
| **Distributed chunk rendering** | **High** (~3–4 weeks) | Requires orchestrator, retry logic, chunk storage, and final concat. Only needed if Nom targets cloud/serverless rendering. |
| **End-to-end GPU → MP4 demo** | **Medium** (~2–3 weeks total) | Assuming wgpu render target is ready: pipe frames into FFmpeg, mux static audio, output MP4/WebM. Matches ROADMAP item: *"Video GPU → FFmpeg parallel encode"* and *"Video-compose demo: paragraph → 10-second MP4"*. |

### Recommended Next Steps

1. **Prototype the FFmpeg pipe** first: write a minimal Rust binary that spawns FFmpeg with `image2pipe`, feeds 300 synthetic frames, and outputs a valid MP4. This validates the `prespawnFfmpeg` pattern in isolation.
2. **Map wgpu render target → `image::RgbaImage`** and feed the bytes into the pipe.
3. **Add `ensureFramesInOrder` equivalent** so parallel wgpu workers can write into the same FFmpeg stdin safely.
4. **Expand `VideoEncoder` stub** in `video_encode.rs` to own the FFmpeg child process and argument generation.
5. **Defer distributed rendering** until the local pipeline is solid.

---

*Audit completed 2026-04-19. No production code was written during this analysis.*
