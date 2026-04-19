# FFmpeg Pattern Audit for Nom

**Date:** 2026-04-19
**Source:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\ffmpeg` (main branch)
**Target Nom file:** `nom-canvas/crates/nom-compose/src/video_encode.rs`

---

## 1. Pattern Summary

FFmpeg's architecture is built around four layered libraries that communicate through a small set of refcounted opaque structs (`AVFrame`, `AVPacket`, `AVFormatContext`, `AVCodecContext`, `AVFilterGraph`). The most important pattern for Nom is the **send/receive state-machine API** used in `libavcodec`: `avcodec_send_frame()` / `avcodec_receive_packet()` for encoding, and `avcodec_send_packet()` / `avcodec_receive_frame()` for decoding. This decouples input from output, lets codecs buffer internally (e.g., for B-frames), and requires explicit EOF flushing by sending a NULL input then draining with a loop until `AVERROR_EOF`. Another critical pattern is the **filter-graph DSL** in `libavfilter`. A graph is parsed from a human-readable string (e.g., `"[in]scale=iw/2:-1[out]"`) by `avfilter_graph_parse2()` in `graphparser.c`, then instantiated via `avfilter_graph_create_filter()`. Links between filters negotiate pixel formats, sample rates, and channel layouts automatically through `merge_formats_internal()` in `formats.c`. Finally, the **scheduler DAG** in `fftools/ffmpeg_sched.c` models the entire transcode pipeline as a directed acyclic graph of demuxers, decoders, filtergraphs, encoders, and muxers, using `ThreadQueue` and `SyncQueue` for lock-free inter-thread handoff. This is the blueprint for how Nom could wire video sources -> GPU filters -> encoders -> muxers in parallel.

## 2. Key Source Files

| File | What it contains |
|------|------------------|
| `fftools/ffmpeg.c` | Main CLI entry point (`main()`), benchmark harness, signal handling. |
| `fftools/ffmpeg_opt.c` | Option parsing with `OptionDef` tables, `GlobalOptionsContext`, `uninit_options()`, per-stream specifier handling (`OPT_FLAG_PERSTREAM`). |
| `fftools/cmdutils.c` / `cmdutils.h` | Generic CLI utilities: `opt_default()`, `parse_number()`, `init_dynload()`, `OptionType` enum (`OPT_TYPE_FUNC`, `OPT_TYPE_BOOL`, `OPT_TYPE_STRING`, etc.). |
| `fftools/ffmpeg_sched.c` / `ffmpeg_sched.h` | Transcode scheduler DAG. Defines `SchedulerNodeType` (`SCH_NODE_TYPE_DEMUX`, `SCH_NODE_TYPE_DEC`, `SCH_NODE_TYPE_ENC`, `SCH_NODE_TYPE_MUX`, `SCH_NODE_TYPE_FILTER_IN`, `SCH_NODE_TYPE_FILTER_OUT`), `ThreadQueue`, `SyncQueue`, `SchEnc` with lazy open callback, `SchDec`. |
| `fftools/ffmpeg_filter.c` | Filter graph setup in the CLI: `FilterGraphPriv`, `InputFilterPriv`, `OutputFilter`, binding decoder outputs to `buffersrc` and encoder inputs to `buffersink`. |
| `libavfilter/graphparser.c` | Filter graph string parser: `avfilter_graph_parse2()`, `parse_link_name()`, `AVFilterInOut` linked-list management, `avfilter_graph_segment_parse()`. |
| `libavfilter/avfiltergraph.c` | Graph lifecycle: `avfilter_graph_alloc()`, `avfilter_graph_create_filter()`, `avfilter_graph_free()`, filter list management. |
| `libavfilter/formats.c` | Format negotiation: `merge_formats_internal()`, `MERGE_FORMATS()` macro, chroma/alpha loss protection, `AVFilterFormats` intersection. |
| `libavfilter/buffersrc.c` | Source filter (`BufferSourceContext`) that accepts raw `AVFrame`s into the graph; param-change validation macros. |
| `libavfilter/buffersink.c` | Sink filter (`BufferSinkContext`) that pulls processed frames out; `av_buffersink_get_frame()`. |
| `libavformat/mux.c` | Muxing core: `avformat_alloc_output_context2()`, `frac_init()`, `frac_add()`, `AVOutputFormat` selection, stream interleaving. |
| `libavformat/demux.c` | Demuxing core: `avformat_open_input()` helpers, `av_probe_input_format3()`, `set_codec_from_probe_data()`, `find_probe_decoder()`. |
| `libavformat/avformat.h` | Public API for container I/O: `AVFormatContext`, `AVStream`, `AVInputFormat`, `AVOutputFormat`. |
| `libavcodec/encode.c` | Encoding helpers: `ff_alloc_packet()`, `ff_get_encode_buffer()`, `avcodec_default_get_encode_buffer()`, `EncodeContext`. |
| `libavcodec/decode.c` | Decoding helpers: `DecodeContext`, draining logic (`draining_started`), PTS correction, `apply_param_change()`. |
| `libavcodec/avcodec.h` | Public codec API: `avcodec_send_packet()`, `avcodec_receive_frame()`, `avcodec_send_frame()`, `avcodec_receive_packet()`, `AVCodecContext`. |
| `libavcodec/codec_desc.c` | Static `codec_descriptors[]` table mapping `AVCodecID` -> name, `AVMediaType`, properties (`AV_CODEC_PROP_LOSSY`, `AV_CODEC_PROP_INTRA_ONLY`), profiles. |

## 3. Nom Mapping

**Current Nom stub:** `nom-canvas/crates/nom-compose/src/video_encode.rs`

| FFmpeg concept | Nom equivalent (current or proposed) |
|----------------|--------------------------------------|
| `AVCodecID` / `AVCodecDescriptor` | `VideoCodec` enum (`H264`, `H265`, `Vp9`, `Av1`). Should expand to mirror `codec_descriptors[]` properties (lossy, intra-only, reorder). |
| `AVFrame` | `VideoFrame` (width, height, frame_index, pixel_count). Missing: pixel format, time_base, PTS, `hw_frames_ctx`. |
| `avcodec_send_frame()` / `avcodec_receive_packet()` | `VideoEncoder::encode_frame()` is currently a no-op counter. Should become an async state machine that feeds an `AVFrame` equivalent and drains packets. |
| `AVCodecContext` | `VideoEncoder` struct should grow to hold codec parameters (bitrate, GOP, preset), matching `AVCodecContext` fields. |
| `avfilter_graph_parse2()` + `avfilter_graph_create_filter()` | Nom has no filter graph yet. A `FilterGraph` DSL parser (Nomx extension?) could reuse the bracket-link syntax: `[in]scale=...;split[out0][out1]`. |
| `buffersrc` / `buffersink` | New `FilterGraphInput` / `FilterGraphOutput` wrappers that push/pull `VideoFrame`s into/out of a graph. |
| `GpuVideoEncoder` | Should map to either (a) multiple `AVCodecContext`s with hardware frames, or (b) an `avfilter_graph` with `AVFILTER_THREAD_SLICE` and a single `hw_frames_ctx`. |
| `avformat_alloc_output_context2()` + `avformat_write_header()` | New muxer builder in Nom that selects container by filename extension (`.mp4`, `.webm`) and writes stream headers. |
| `frac_init()` / `frac_add()` | Timestamp management for `VideoFrame` -- Nom will need rational-time arithmetic to avoid A/V desync. |

## 4. Licensing / Complexity Notes

- **License:** The core APIs in `libavcodec`, `libavformat`, and `libavfilter` are **LGPLv2.1+**. Optional GPL parts (certain filters in `libavfilter/vf_*.c`, x264/x265 linkage) are **not** enabled by default and can be omitted. If Nom links only LGPL symbols, it can remain permissive-friendly.
- **Build complexity:** FFmpeg uses a 319 K-line POSIX `configure` shell script with extensive feature probing. Static linking on Windows requires either a prebuilt LGPL binary or a cross-compilation toolchain (MSYS2/MinGW). The `configure` script is not easily embeddable as a Cargo build script.
- **ABI stability:** The C ABI is stable across major versions, so Nom could safely call into prebuilt shared libraries (`avcodec-61.dll`, `avformat-61.dll`, `avfilter-10.dll`) via `bindgen` + `libloading` or the `ffmpeg-next`/`ffmpeg-sys` crates.
- **Hardware acceleration:** `AVHWFramesContext`, `AVHWDeviceContext`, and `hw_frames_ctx` propagation through filters are among the most error-prone areas. Any GPU path will need careful validation.
- **Thread safety:** The scheduler shows that encoders/decoders are opened lazily from the first frame inside the worker thread (`SchEnc.open_cb`). Nom should copy this lazy-open pattern to avoid deadlocks between format negotiation and thread startup.

## 5. Adoption Effort Estimate

| Task | Effort | Blockers |
|------|--------|----------|
| Bind FFmpeg libraries (`avcodec`, `avformat`, `avfilter`, `avutil`) into Nom build | **2-3 days** | `configure` / cross-compilation on Windows; choosing `ffmpeg-sys` vs. prebuilt DLLs vs. custom `bindgen`. |
| Replace `VideoEncoder` stub with real `avcodec` send/receive loop | **3-4 days** | Managing `AVFrame` lifetime, pixel buffer allocation, PTS/DTS bookkeeping, codec parameter validation. |
| Implement filter-graph builder + Nomx DSL parser | **5-7 days** | Replicating `graphparser.c` link-label syntax, format negotiation (`merge_formats_internal`), auto-insertion of conversion filters (`scale`, `aresample`). |
| Add muxer wrapper (`avformat` output) | **2-3 days** | Stream interleaving, fractional timestamp (`FFFrac`) logic, container format guessing from filename. |
| Hardware/GPU acceleration path | **5-10 days** | `AVHWFramesContext` setup, `hw_frames_ctx` passing through filters, device-type selection (CUDA, D3D11, Vulkan). |
| Scheduler / parallel encoding (multiple streams) | **4-6 days** | Threading model (`ThreadQueue`, `SyncQueue`), lazy encoder open, EOF flushing across threads. |
| **Total realistic adoption** | **~3-5 weeks** (one engineer) | Build system integration and hardware context are the highest-risk items. The pure software encode path could be functional in ~1.5 weeks if using existing Rust FFmpeg wrapper crates. |
