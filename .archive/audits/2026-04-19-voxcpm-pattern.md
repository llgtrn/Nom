# VoxCPM Pattern Audit Report

**Date:** 2026-04-19  
**Source:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\VoxCPM-main`  
**Analyst:** Pattern-extraction analyst (read-only audit)  
**Scope:** Controllable TTS architecture, voice cloning, inference pipeline, conditioning, audio output.

---

## 1. Pattern Summary

VoxCPM (latest: **VoxCPM2**) is a **tokenizer-free, diffusion-autoregressive TTS** system. It does not use discrete audio tokens (no SoundStream / EnCodec vocabularies). Instead it operates directly in the **continuous latent space of a causal AudioVAE**, using a four-stage pipeline:

1. **LocEnc** (`VoxCPMLocEnc`) � patches raw audio VAE latents into LM-compatible embeddings.
2. **TSLM** (`MiniCPMModel` base LM) � autoregressive text-semantic language model (MiniCPM-4 backbone, ~2B params).
3. **RALM** (`MiniCPMModel` residual LM) � residual acoustic LM that fuses TSLM outputs with re-encoded audio patches.
4. **LocDiT** (`VoxCPMLocDiTV2` + `UnifiedCFM`) � local diffusion transformer that performs **flow-matching** (Euler solver, classifier-free guidance) to generate the next audio feature patch.

Generation is **autoregressive patch-by-patch**: the LM hidden state at each step is projected into the DiT conditioner (`mu`), noise is initialized, flow-matching integrates over `n_timesteps` (default 10), and the resulting patch is fed back as the next LM input via `forward_step` with KV caches. A stop-predictor head (`stop_proj` ? `SiLU` ? `stop_head`) terminates generation.

**Voice cloning** works by encoding a reference WAV through the AudioVAE and prefixing it with special tokens (`ref_audio_start_token=103`, `ref_audio_end_token=104`) so the LM sees the reference timbre without confusing it with the target text. **Controllable generation** prepends a natural-language voice description in parentheses, e.g. `(A warm young woman)Hello...`.

**Streaming** is supported: each autoregressive step yields a latent chunk; the AudioVAE decodes the last `streaming_prefix_len` patches to produce playable PCM.

---

## 2. Key Source Files

| File | Role | Key Classes / Functions |
|------|------|------------------------|
| `src/voxcpm/core.py` | High-level Python API (`VoxCPM` class). Handles model loading, denoising, text normalization, prompt caching, retry logic, and streaming/non-streaming dispatch. | `VoxCPM.__init__`, `VoxCPM.from_pretrained`, `VoxCPM._generate`, `VoxCPM.generate_streaming` |
| `src/voxcpm/model/voxcpm.py` | VoxCPM v1 model definition. Contains `VoxCPMModel`, `VoxCPMConfig`, `LoRAConfig`, and the core inference loop `_inference`. | `VoxCPMModel._inference`, `VoxCPMModel._generate_with_prompt_cache`, `VoxCPMModel.build_prompt_cache`, `VoxCPMModel.from_local` |
| `src/voxcpm/model/voxcpm2.py` | **VoxCPM2** model definition. Adds reference-isolation tokens (`ref_audio_start_token`, `ref_audio_end_token`), `fusion_concat_proj`, VAD trimming (`_trim_audio_silence_vad`), and `_make_ref_prefix`. | `VoxCPM2Model._generate`, `VoxCPM2Model.build_prompt_cache`, `VoxCPM2Model._encode_wav`, `VoxCPM2Model._make_ref_prefix`, `VoxCPM2Model._inference` |
| `src/voxcpm/modules/audiovae/audio_vae.py` | V1 causal AudioVAE (16kHz). Encoder/decoder with Snake1d activations, weight-norm causal convolutions, and depthwise separable blocks. | `AudioVAE`, `CausalEncoder`, `CausalDecoder`, `CausalResidualUnit`, `Snake1d` |
| `src/voxcpm/modules/audiovae/audio_vae_v2.py` | **V2 AudioVAE** (16kHz encode ? 48kHz decode). Adds `SampleRateConditionLayer` for multi-rate decoding, asymmetric encoder/decoder rates, and `sr_bin_boundaries` conditioning. | `AudioVAEV2`, `SampleRateConditionLayer`, `CausalDecoderBlock` |
| `src/voxcpm/modules/locdit/local_dit_v2.py` | Local Diffusion Transformer (DiT) estimator. Uses `MiniCPMModel` as the transformer backbone, with sinusoidal timestep embeddings and prefix conditioning. | `VoxCPMLocDiTV2`, `SinusoidalPosEmb`, `TimestepEmbedding` |
| `src/voxcpm/modules/locdit/unified_cfm.py` | **Flow-matching solver**. Implements training loss (`compute_loss`) and Euler inference (`solve_euler`) with classifier-free guidance (`cfg_value`) and optimized zero-star scaling. | `UnifiedCFM`, `UnifiedCFM.solve_euler`, `UnifiedCFM.forward` (inference), `UnifiedCFM.compute_loss` |
| `src/voxcpm/modules/locenc/local_encoder.py` | Local encoder that converts audio VAE latent patches into LM embeddings via a CLS-style transformer. | `VoxCPMLocEnc` |
| `src/voxcpm/modules/minicpm4/model.py` | MiniCPM-4 transformer backbone. RMSNorm, GQA attention with RoPE (`MiniCPMLongRoPE`), `forward_step` for cached single-token generation, and `StaticKVCache`. | `MiniCPMModel`, `MiniCPMAttention`, `MiniCPMLongRoPE`, `MiniCPMDecoderLayer` |
| `src/voxcpm/modules/layers/lora.py` | LoRA injection for fine-tuning. Wraps `nn.Linear` into `LoRALinear`, supporting `torch.compile` via buffer-based scaling. | `LoRALinear`, `apply_lora_to_named_linear_modules` |
| `src/voxcpm/zipenhancer.py` | Optional pre-processing denoiser using ModelScope ZipEnhancer. | `ZipEnhancer.enhance` |
| `app.py` | Gradio demo showing the three public modes: Voice Design, Controllable Cloning, Ultimate Cloning. | `VoxCPMDemo.generate_tts_audio`, `VoxCPMDemo.prompt_wav_recognition` |

---

## 3. Nom Mapping

> **Target:** `nom-compose/src/voice.rs` was referenced in the task brief, but **no such file exists** in the current Nom tree. The canonical audio composition surface is:
> - `nom-canvas/crates/nom-compose/src/backends/audio.rs` � artifact encoding (WAV/FLAC/OGG/MP3)
> - `nom-canvas/crates/nom-compose/src/audio_encode.rs` � `AudioBuffer`, `AudioEncoder`, `RodioBackend`

### What to map

| VoxCPM Concept | Nom Equivalent / Integration Point | Notes |
|----------------|-----------------------------------|-------|
| **TTS engine entrypoint** | New backend module (e.g. `backends/voice.rs`) or a `VoiceCompose` pipeline stage | Should accept text, optional reference audio path/bytes, and optional control instruction string. |
| **Audio output (PCM f32)** | `audio_encode.rs::AudioBuffer` + `backends/audio.rs::AudioBackend` | VoxCPM outputs `np.ndarray` float32 PCM at `sample_rate` (48kHz for V2). This maps directly to `AudioBuffer { samples: Vec<f32>, sample_rate, channels: 1 }`. |
| **Streaming chunks** | `nom-compose/src/streaming.rs` | VoxCPM2 yields per-step chunks in `generate_streaming`. These can be appended to an `AudioBuffer` or emitted as `StreamingResult` events. |
| **Reference audio ingestion** | `audio_encode.rs::AudioBuffer` (decode) ? pass path to VoxCPM | Nom already has PCM buffer abstractions. The reference audio path or decoded bytes would be passed to the inference wrapper. |
| **Voice description / control** | Text preprocessing in the new backend | VoxCPM expects control instructions as a parenthesized prefix: `(description)target_text`. Nom should assemble this before calling inference. |
| **Artifact storage** | `ArtifactStore::write` (used by `AudioBackend::compose`) | After TTS generation, PCM samples are encoded to WAV/MP3 via existing `AudioBackend` and stored by hash. |
| **Progress / cancellation** | `ProgressSink::emit(ComposeEvent::Progress)` + `CancellationToken` | VoxCPM inference is iterative (one Euler solve per patch). Progress can be reported per patch or per decoded chunk. |

### Suggested Rust interface sketch

```rust
// In a new nom-compose/src/backends/voice.rs (or voice_gen.rs)
pub struct VoiceInput {
    pub text: String,
    pub control_description: Option<String>, // e.g. "A warm young woman"
    pub reference_audio: Option<AudioBuffer>, // cloned from reference WAV
    pub reference_text: Option<String>,      // for Ultimate Cloning / continuation
    pub cfg_value: f32,                      // default 2.0
    pub inference_steps: u32,                // default 10
    pub streaming: bool,
}

pub struct VoiceBackend;
impl VoiceBackend {
    pub fn compose(
        input: VoiceInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> AudioBlock {
        // 1. Assemble control prefix if present
        // 2. Dispatch to VoxCPM inference (Python server / ONNX / tch-rs)
        // 3. Receive Vec<f32> PCM at 48000 Hz
        // 4. Wrap in AudioBuffer and delegate to AudioBackend::compose
    }
}
```

---

## 4. Licensing / Complexity Notes

### Licensing
- **Code & weights:** Apache-2.0 (explicit in `LICENSE`, headers in `voxcpm.py` / `voxcpm2.py`).
- **Commercial use:** Permitted without restriction.
- **Third-party deps:** PyTorch (BSD), Transformers (Apache-2.0), ModelScope ZipEnhancer (Apache-2.0), Gradio (Apache-2.0). All permissive.

### Complexity & Constraints
- **Model size:** 2B parameters; ~8 GB VRAM at bfloat16 inference.
- **Runtime dependencies:** PyTorch = 2.5, CUDA = 12.0, `transformers`, `safetensors`, `einops`, `torchaudio`, `librosa`. The core is **deeply Python/PyTorch** � no native Rust implementation exists in the upstream repo.
- **Inference cost:** ~0.3 RTF on RTX 4090 (PyTorch), ~0.13 RTF with community Nano-vLLM engine. Each audio patch requires a full Euler flow-matching solve (default 10 steps) through the DiT.
- **Streaming caveat:** Streaming yields chunks, but each chunk requires the last `streaming_prefix_len` patches for causal VAE decode context. Latency is per-patch, not per-token.
- **torch.compile:** The model aggressively uses `torch.compile` on `forward_step`, `feat_encoder`, and `feat_decoder.estimator` for speed. This requires Triton and CUDA.
- **LoRA ecosystem:** The upstream supports LoRA hot-swapping (`load_lora_weights`, `set_lora_enabled`, `reset_lora_weights`). Any integration should preserve this if speaker customization is needed.
- **Community ports exist:** `VoxCPM.cpp` (GGML/GGUF), `VoxCPM-ONNX`, `voxcpm_rs` (Rust re-impl). These could lower adoption effort if a pure-Rust or ONNX Runtime path is preferred over embedding Python.

---

## 5. Adoption Effort Estimate

| Approach | Effort | Pros | Cons |
|----------|--------|------|------|
| **A. Python inference server (FastAPI/gRPC) called from Rust** | **Medium** (~2�3 weeks) | Minimal model-porting risk; reuse upstream `core.py` exactly; streaming over HTTP/gRPC is straightforward. | Requires shipping Python + CUDA runtime; adds process boundary latency; deployment complexity. |
| **B. ONNX Runtime (community `VoxCPM-ONNX`)** | **Medium�High** (~3�5 weeks) | No Python runtime; pure Rust/C++ inference; better for edge deployment. | ONNX export may not cover LoRA hot-swapping, streaming, or the full flow-matching loop. Needs validation against VoxCPM2 (latest). |
| **C. `tch-rs` (libtorch bindings) direct port** | **High** (~6�10 weeks) | Native Rust; no separate Python process; tight integration with Nom. | `tch-rs` lacks `transformers` ecosystem (tokenizers, RoPE cache, SDPA). Would need to re-implement `MiniCPMModel`, KV cache, and the entire flow-matching loop in Rust. |
| **D. GGML / `VoxCPM.cpp` bindings** | **Medium�High** (~3�5 weeks) | CPU/CUDA/Vulkan; good for low-VRAM deployments; quantization support. | May lag upstream features (VoxCPM2, 48kHz, LoRA, streaming). Needs FFI bindings. |

### Recommended path for Nom
1. **Short term:** Wrap the upstream Python package in a local HTTP/gRPC service (or use the community Nano-vLLM server). Nom calls it via `reqwest` or a thin async client. PCM output is fed into existing `AudioBackend::compose`.
2. **Long term:** Evaluate the `VoxCPM-ONNX` export for the inference loop. If it supports CFG, flow-matching, and the causal AudioVAE decode path, migrate to ONNX Runtime Rust bindings to eliminate the Python dependency.

### Key integration files to touch in Nom
- **New:** `nom-canvas/crates/nom-compose/src/backends/voice.rs` (or `voice_gen.rs`) � TTS orchestration.
- **Existing:** `nom-canvas/crates/nom-compose/src/backends/audio.rs` � reuse for final artifact encoding.
- **Existing:** `nom-canvas/crates/nom-compose/src/audio_encode.rs` � reuse `AudioBuffer` as the PCM interchange type.
- **Existing:** `nom-canvas/crates/nom-compose/src/streaming.rs` � hook for streaming chunk delivery.

---

*Report generated from full source read of VoxCPM-main (commit range c. 2026-04). All class and function names are cited directly from the upstream source.*
