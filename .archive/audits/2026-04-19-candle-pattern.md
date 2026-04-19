# Candle Pattern Audit — Nom In-Process ML Adoption

**Date:** 2026-04-19
**Auditor:** Pattern-extraction analyst
**Source repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\candle`
**Target:** `nom-canvas/crates/nom-compiler-bridge/src/candle_adapter.rs`
**ROADMAP ref:** D10 — UC-CANDLE (`CandleAdapter` stubbed → real `BackendDevice::Cpu` + `ReActLlmFn` impl)

---

## 1. Pattern Summary

### 1.1 Architecture Overview

Candle is a pure-Rust ML framework (no Python/PyTorch runtime) built on three layers:

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| **Core** | `candle-core` | `Tensor`, `Device`, `BackendDevice`/`BackendStorage` traits, memory management, CPU/CUDA/Metal backends |
| **NN** | `candle-nn` | Layer definitions (`Linear`, `Embedding`, `RmsNorm`), `VarBuilder` for weight loading, optimizers (`AdamW`, `SGD`) |
| **Transformers** | `candle-transformers` | Pre-built model families (LLaMA, Mistral, Gemma, Phi, Qwen, etc.), generation pipelines (`LogitsProcessor`), quantized inference |

### 1.2 Backend Abstraction

**`BackendDevice`** (`candle-core/src/backend.rs`, line 141) defines device behavior:

```rust
pub trait BackendDevice: Sized + std::fmt::Debug + Clone {
    type Storage: BackendStorage;
    fn new(_: usize) -> Result<Self>;
    fn zeros_impl(&self, _shape: &Shape, _dtype: DType) -> Result<Self::Storage>;
    unsafe fn alloc_uninit(&self, _shape: &Shape, _dtype: DType) -> Result<Self::Storage>;
    fn storage_from_slice<T: WithDType>(&self, _: &[T]) -> Result<Self::Storage>;
    fn rand_uniform(&self, _: &Shape, _: DType, _: f64, _: f64) -> Result<Self::Storage>;
    fn rand_normal(&self, _: &Shape, _: DType, _: f64, _: f64) -> Result<Self::Storage>;
    fn set_seed(&self, _: u64) -> Result<()>;
    fn synchronize(&self) -> Result<()>;
}
```

**`BackendStorage`** (same file, line 6) defines tensor operations per backend:

```rust
pub trait BackendStorage: Sized {
    type Device: BackendDevice;
    fn try_clone(&self, _: &Layout) -> Result<Self>;
    fn dtype(&self) -> DType;
    fn device(&self) -> &Self::Device;
    fn to_cpu_storage(&self) -> Result<CpuStorage>;
    fn affine(&self, _: &Layout, _: f64, _: f64) -> Result<Self>;
    fn unary_impl<B: UnaryOpT>(&self, _: &Layout) -> Result<Self>;
    fn binary_impl<B: BinaryOpT>(&self, _: &Self, _: &Layout, _: &Layout) -> Result<Self>;
    fn matmul(&self, _: &Self, _: (usize, usize, usize, usize), _: &Layout, _: &Layout) -> Result<Self>;
    fn reduce_op(&self, _: ReduceOp, _: &Layout, _: &[usize]) -> Result<Self>;
    // ... conv, pool, gather, scatter, index_select, etc.
}
```

**`Device` enum** (`candle-core/src/device.rs`, line 16) is the user-facing handle:

```rust
pub enum Device {
    Cpu,
    Cuda(crate::CudaDevice),
    Metal(crate::MetalDevice),
}
```

All tensor creation (`Tensor::zeros`, `Tensor::randn`, etc.) dispatch through `Device` to the concrete backend. CPU is always available; CUDA and Metal are gated by feature flags (`cuda`, `metal`).

### 1.3 Tensor & Memory Model

**`Tensor`** (`candle-core/src/tensor.rs`, line 68) is refcounted via `Arc<Tensor_>`:

```rust
pub struct Tensor(Arc<Tensor_>);
pub struct Tensor_ {
    id: TensorId,
    storage: Arc<RwLock<Storage>>,
    layout: Layout,
    op: BackpropOp,       // tracks graph for backprop
    is_variable: bool,
    dtype: DType,
    device: Device,
}
```

**`Storage`** (`candle-core/src/storage.rs`, line 10) is a backend-tagged enum:

```rust
pub enum Storage {
    Cpu(CpuStorage),
    Cuda(CudaStorage),
    Metal(MetalStorage),
}
```

`CpuStorage` (`candle-core/src/cpu_backend/mod.rs`, line 22) is a dtype-tagged `Vec<T>`:

```rust
pub enum CpuStorage {
    U8(Vec<u8>), U32(Vec<u32>), I16(Vec<i16>), I32(Vec<i32>),
    I64(Vec<i64>), BF16(Vec<bf16>), F16(Vec<f16>), F32(Vec<f32>),
    F64(Vec<f64>), F8E4M3(Vec<F8E4M3>), ...
}
```

**Key memory insight:** Tensors are cheap to clone (Arc bump). The actual buffer is behind `Arc<RwLock<Storage>>`, so views/strides share storage. CPU backend uses `rayon` for parallel loops and AVX2/NEON/SIMD128 vectorized kernels (`candle-core/src/cpu/`). GPU backends use CUDA/Metal kernels compiled at build time (`candle-kernels`, `candle-metal-kernels`).

### 1.4 Model Building Patterns

**`Module` trait** (`candle-core/src/lib.rs`, line 148) is the universal forward interface:

```rust
pub trait Module {
    fn forward(&self, xs: &Tensor) -> Result<Tensor>;
}
```

**`VarBuilder`** (`candle-nn/src/var_builder.rs`, line 36) decouples model definition from weight loading:

```rust
pub type VarBuilder<'a> = VarBuilderArgs<'a, Box<dyn SimpleBackend + 'a>>;
```

Backends for `VarBuilder`:
- `MmapedSafetensors` — memory-mapped `.safetensors` files (no full RAM copy)
- `BufferedSafetensors` / `SliceSafetensors` — in-memory safetensor buffers
- `PthTensors` — PyTorch `.pth`/`.bin` pickle files
- `VarMap` — trainable variables (creates on first access)
- `Zeros` — dummy backend for shape inference
- `ShardedSafeTensors` — tensor-parallel sharding

Example: loading LLaMA (`candle-transformers/src/models/llama.rs`, line 515):

```rust
pub fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
    let wte = embedding(cfg.vocab_size, cfg.hidden_size, vb.pp("model.embed_tokens"))?;
    let lm_head = linear(cfg.hidden_size, cfg.vocab_size, vb.pp("lm_head"))?;
    let ln_f = RmsNorm::new(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("model.norm"))?;
    let blocks: Vec<_> = (0..cfg.num_hidden_layers)
        .map(|i| Block::load(vb.pp(format!("model.layers.{i}")), cfg).unwrap())
        .collect();
    Ok(Self { wte, blocks, ln_f, lm_head })
}
```

**Pattern:** `vb.pp("path")` drills into nested namespaces. Models are pure structs of `Linear`/`Embedding`/`RmsNorm` layers. No macro magic — just plain Rust structs implementing `Module`.

### 1.5 In-Process Inference (No Python)

Candle eliminates Python entirely:

1. **Weight loading** — directly reads Hugging Face `safetensors` or GGUF (llama.cpp format) via `candle::safetensors::MmapedSafetensors` or `candle::quantized::gguf_file::Content`.
2. **Tokenization** — uses the `tokenizers` crate (Rust port of HF tokenizers) or simple BPE/WordPiece implementations in examples.
3. **Forward pass** — pure Rust `Module::forward` calls dispatch to CPU/GPU kernels.
4. **Sampling** — `LogitsProcessor` (`candle-transformers/src/generation/mod.rs`, line 20) implements temperature, top-k, top-p, argmax in-process:

```rust
pub struct LogitsProcessor {
    rng: rand::rngs::StdRng,
    sampling: Sampling,
}
pub enum Sampling {
    ArgMax,
    All { temperature: f64 },
    TopK { k: usize, temperature: f64 },
    TopP { p: f64, temperature: f64 },
    TopKThenTopP { k: usize, p: f64, temperature: f64 },
    GumbelSoftmax { temperature: f64 },
}
```

A typical text-generation loop (pattern from examples):

```rust
let mut logits_processor = LogitsProcessor::new(seed, temperature, top_p);
let mut tokens = tokenizer.encode(prompt)?;
for _ in 0..max_tokens {
    let input = Tensor::new(&tokens, device)?;
    let logits = model.forward(&input, index_pos, &mut cache)?;
    let next_token = logits_processor.sample(&logits)?;
    tokens.push(next_token);
    if Some(next_token) == eos_token_id { break; }
}
```

### 1.6 Quantized Inference

For CPU-bound deployments, Candle supports GGUF quantization:

- **`QTensor`** (`candle-core/src/quantized/`) — 4-bit/8-bit quantized tensors.
- **`quantized_var_builder::VarBuilder`** (`candle-transformers/src/quantized_var_builder.rs`) — loads GGUF files directly.
- **`quantized_nn::Linear`** (`candle-transformers/src/quantized_nn.rs`) — `QMatMul` layer that dequantizes-on-the-fly or uses quantized matmul kernels.

This is the recommended path for Nom's CPU-first `BackendDevice::Cpu` target, as it reduces memory footprint by ~4x and speeds up inference on consumer hardware.

---

## 2. Key Source Files

### candle-core (backend, tensor, memory)

| File | Lines | What it defines |
|------|-------|-----------------|
| `src/backend.rs` | 174 | `BackendDevice`, `BackendStorage` traits |
| `src/device.rs` | 502 | `Device` enum, `DeviceLocation`, `NdArray` |
| `src/storage.rs` | 849 | `Storage` enum (CPU/CUDA/Metal dispatch) |
| `src/tensor.rs` | 3116 | `Tensor`, `Tensor_`, `TensorId`, all tensor ops |
| `src/cpu_backend/mod.rs` | 3284 | `CpuStorage`, `CpuDevice`, CPU kernel implementations |
| `src/cpu/mod.rs` | 242 | SIMD traits (`Cpu`, `CpuF16`, `CpuBF16`) and `vec_dot_f32` |
| `src/cpu/avx.rs` | — | AVX2 `CurrentCpu` implementation |
| `src/cpu/neon.rs` | — | ARM NEON `CurrentCpu` implementation |
| `src/safetensors.rs` | 644 | `MmapedSafetensors`, `BufferedSafetensors`, `SliceSafetensors`, `Load` trait |
| `src/quantized/` | — | `QTensor`, GGUF reader, quantized matmul |
| `src/lib.rs` | 177 | `Module`, `ModuleT`, crate re-exports |

### candle-nn (layers, builder, optimizers)

| File | Lines | What it defines |
|------|-------|-----------------|
| `src/lib.rs` | 63 | Crate exports, re-exports `Module` from `candle-core` |
| `src/linear.rs` | 114 | `Linear` struct, `linear()`, `linear_no_bias()`, `linear_b()` |
| `src/var_builder.rs` | 906 | `VarBuilder`, `VarBuilderArgs`, `Backend`/`SimpleBackend` traits, safetensors/VarMap backends |
| `src/activation.rs` | 109 | `Activation` enum (Gelu, Silu, Swiglu, etc.), `PReLU` |
| `src/layer_norm.rs` | — | `LayerNorm`, `RmsNorm` |
| `src/embedding.rs` | — | `Embedding` |
| `src/ops.rs` | — | `softmax_last_dim`, `sigmoid`, `swiglu`, `rms_norm`, `dropout` |
| `src/optim.rs` | — | `AdamW`, `SGD`, `Optimizer` trait |
| `src/sampling.rs` | — | `gumbel_softmax` |

### candle-transformers (models, generation, quantization)

| File | Lines | What it defines |
|------|-------|-----------------|
| `src/models/llama.rs` | 534 | `Llama`, `LlamaConfig`, `Cache`, `CausalSelfAttention`, `Mlp`, `Block` |
| `src/models/mistral.rs` | 467 | `Model` (Mistral/Mixtral), `Attention`, `DecoderLayer`, `RotaryEmbedding` |
| `src/models/gemma.rs` | — | `Gemma` model |
| `src/models/phi.rs` | — | `Phi` model |
| `src/models/qwen3.rs` | — | `Qwen3` model |
| `src/models/mod.rs` | — | Re-exports all model families |
| `src/generation/mod.rs` | 158 | `LogitsProcessor`, `Sampling` enum |
| `src/pipelines/text_generation.rs` | 1 | Stub (actual pipeline logic lives in examples) |
| `src/quantized_var_builder.rs` | 104 | GGUF `VarBuilder` |
| `src/quantized_nn.rs` | 126 | Quantized `Linear`, `Embedding`, `RmsNorm` |
| `src/utils.rs` | — | `build_causal_mask`, `repeat_kv` |

### candle-examples (end-to-end patterns)

| File | What it shows |
|------|---------------|
| `src/lib.rs` | `device(cpu: bool)`, `hub_load_safetensors`, `hub_load_local_safetensors` |
| `examples/llama/` | Full text-generation binary: tokenizer -> model load -> sampling loop |
| `examples/phi/` | Phi-3 inference with `VarBuilder::from_mmaped_safetensors` |

---

## 3. Nom Mapping

### 3.1 Current State

`nom-canvas/crates/nom-compiler-bridge/src/candle_adapter.rs` (117 lines) contains:

- `BackendDevice` enum — `Cpu` / `Cuda(usize)` — **shadows** candle's `Device` but is a local stub.
- `ModelConfig` — holds `model_id`, `device`, `max_tokens`.
- `InferenceFn` trait — `infer(&self, prompt: &str) -> Result<String, String>` — mirrors `ReActLlmFn`.
- `CandleAdapter` — stub implementation; `generate()` returns hardcoded strings.

### 3.2 Migration Path (stub -> real)

**Step A — Add dependencies to `nom-canvas/crates/nom-compiler-bridge/Cargo.toml`:**

```toml
[dependencies]
candle-core = { version = "0.8", default-features = false }
candle-nn = { version = "0.8", default-features = false }
candle-transformers = { version = "0.8", default-features = false }
tokenizers = "0.21"
# Optional: enable cuda/metal features behind Nom feature flags

[features]
default = []
cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]
metal = ["candle-core/metal", "candle-nn/metal", "candle-transformers/metal"]
```

**Step B — Replace `BackendDevice` with `candle_core::Device`:**

```rust
use candle_core::Device;

// Instead of local enum:
// pub enum BackendDevice { Cpu, Cuda(usize) }
// Use:
// Device::Cpu
// Device::new_cuda(ordinal)?
// Device::new_metal(ordinal)?
```

**Step C — Model loading inside `CandleAdapter::new_cpu(...)`:**

```rust
use candle_core::{DType, Device, Result, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::phi3::{Config, Model};
use candle_transformers::generation::LogitsProcessor;
use tokenizers::Tokenizer;

pub struct CandleAdapter {
    device: Device,
    model: Model,
    tokenizer: Tokenizer,
    logits_processor: LogitsProcessor,
    max_tokens: usize,
}

impl CandleAdapter {
    pub fn new_cpu(model_dir: &std::path::Path) -> Result<Self> {
        let device = Device::Cpu;
        // Load config.json
        let config: Config = serde_json::from_reader(
            std::fs::File::open(model_dir.join("config.json"))?)?;
        // Load safetensors weights via memory-mapped VarBuilder
        let safetensors = unsafe {
            candle_core::safetensors::MmapedSafetensors::multi(&[
                model_dir.join("model.safetensors"),
            ])?
        };
        let vb = VarBuilder::from_backend(
            Box::new(safetensors),
            DType::F32,
            device.clone(),
        );
        let model = Model::new(&config, vb)?;
        let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json")).map_err(|e| {
            candle_core::Error::Msg(format!("tokenizer load failed: {e}"))
        })?;
        let logits_processor = LogitsProcessor::new(299792458, Some(0.8), Some(0.95));
        Ok(Self { device, model, tokenizer, logits_processor, max_tokens: 256 })
    }
}
```

**Step D — Implement `InferenceFn::infer` with real forward pass:**

```rust
impl InferenceFn for CandleAdapter {
    fn infer(&self, prompt: &str) -> Result<String, String> {
        let tokens = self.tokenizer.encode(prompt, true)
            .map_err(|e| e.to_string())?;
        let mut token_ids: Vec<u32> = tokens.get_ids().to_vec();
        let mut output_tokens = Vec::new();
        let mut index_pos = 0;
        for _ in 0..self.max_tokens {
            let input = Tensor::new(&token_ids[index_pos..], &self.device)
                .map_err(|e| e.to_string())?;
            let logits = self.model.forward(&input, index_pos)
                .map_err(|e| e.to_string())?;
            let next_token = self.logits_processor.sample(&logits)
                .map_err(|e| e.to_string())?;
            token_ids.push(next_token);
            output_tokens.push(next_token);
            index_pos = token_ids.len().saturating_sub(1);
            if Some(next_token) == self.tokenizer.token_to_id("<|endoftext|>") {
                break;
            }
        }
        self.tokenizer.decode(&output_tokens, true)
            .map_err(|e| e.to_string())
    }

    fn model_name(&self) -> &str {
        "phi-3-mini"
    }
}
```

**Step E — Quantized / GGUF path (recommended for CPU):**

```rust
use candle_transformers::quantized_var_builder::VarBuilder as QVB;
use candle_transformers::models::quantized_phi3::Model as QPhi3;

let vb = QVB::from_gguf("model.gguf", &device)?;
let model = QPhi3::new(&config, vb)?;
```

This avoids loading full F32 weights into RAM and uses Candle's `QMatMul` kernels.

### 3.3 Trait Alignment Notes

- Nom's `InferenceFn` returns `Result<String, String>`; Candle uses `candle_core::Result<T>` (alias for `std::result::Result<T, candle_core::Error>`). Map with `.map_err(|e| e.to_string())`.
- Nom's stub `BackendDevice` should be **deleted** and replaced by `candle_core::Device` to avoid type friction.
- If Nom needs `Send + Sync` inference, `CandleAdapter` fields must all be `Send + Sync`. Candle's `Model`, `Tokenizer`, and `LogitsProcessor` are all `Send + Sync`.

---

## 4. Licensing / Complexity Notes

### 4.1 License

Candle is **dual-licensed under Apache-2.0 and MIT** (confirmed by `LICENSE-APACHE` and `LICENSE-MIT` in repo root). Nom is free to link, modify, and redistribute under either license. No copyleft concerns.

### 4.2 Complexity & Binary Size

| Concern | Assessment |
|---------|------------|
| **Compile time** | `candle-core` + `candle-nn` + `candle-transformers` adds ~2–3 min to a clean release build. CUDA/Metal features add more due to kernel compilation. |
| **Binary size** | CPU-only baseline: ~5–8 MB added. With quantized models, runtime memory is small (~2–4 GB for 7B Q4). |
| **Dependencies** | Core deps: `safetensors`, `half`, `gemm`, `rayon`, `num-traits`, `rand`, `thiserror`. No Python, no ONNX Runtime, no PyTorch C++ API. |
| **Unsafe** | Limited to CPU SIMD intrinsics, CUDA/Metal FFI, and `mmap` for safetensors. Well-audited by HF team. |
| **MSRV** | Rust 1.82+ (checked `rust-toolchain.toml` in upstream). Nom must align. |

### 4.3 Feature-Gating Strategy

Because CUDA/Metal require native toolchains (NVCC, Xcode), Nom should gate them behind optional Cargo features:

```toml
[features]
default = ["candle-cpu"]
candle-cpu = []
candle-cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]
candle-metal = ["candle-core/metal", "candle-nn/metal", "candle-transformers/metal"]
```

This keeps Nom buildable on stock Windows/Linux without GPU SDKs.

---

## 5. Adoption Effort Estimate

### 5.1 Work Breakdown

| Task | Files | Effort | Risk |
|------|-------|--------|------|
| **1. Add candle deps** | `Cargo.toml` | 30 min | Low |
| **2. Replace stub types** | `candle_adapter.rs` | 1–2 h | Low |
| **3. Implement model loader** | `candle_adapter.rs` | 2–4 h | Medium — path to weights/config must be configurable |
| **4. Implement tokenization + generation loop** | `candle_adapter.rs` | 2–3 h | Medium — need `tokenizers` crate integration |
| **5. Add quantized (GGUF) path** | `candle_adapter.rs` | 2–3 h | Low — Candle has ready `QVB` |
| **6. Write integration tests** | `candle_adapter.rs` tests | 2–3 h | Low — can use tiny model or mock tensors |
| **7. CI feature matrix** | `.github/workflows/` | 1 h | Low — build with/without cuda/metal |

**Total realistic estimate:** **1–2 dev-days** for a working CPU inference adapter.

### 5.2 Recommended Model for First Integration

| Model | Size (Q4) | Why |
|-------|-----------|-----|
| **Phi-3 Mini** | ~2 GB | Small, high quality, good tokenizer, Candle has `phi3.rs` and `quantized_phi3.rs` |
| **Gemma 2B** | ~1.5 GB | Even smaller, permissive license, Candle has `gemma.rs` |
| **Qwen3 0.6B** | ~0.5 GB | Tiny, multilingual, Candle has `qwen3.rs` |

### 5.3 Open Questions for Nom Team

1. **Where do weights live?** Hugging Face cache, bundled asset, or user-provided path? This determines whether to use `hf_hub` or `MmapedSafetensors::multi`.
2. **Which model families?** If Nom needs Vietnamese support, Qwen3 or Gemma are better than Phi-3.
3. **Streaming?** Candle examples decode token-by-token; Nom may want a streaming `infer` variant.
4. **Context window?** Default `max_tokens` in stub is 256. Real models need `max_position_embeddings` clamping.

---

## 6. Quick Reference — Candle Types Nom Will Touch

| Candle type | Nom usage |
|-------------|-----------|
| `candle_core::Device` | Replace `BackendDevice` enum |
| `candle_core::Tensor` | Input/output of `forward` |
| `candle_core::DType` | `F32`, `F16`, `BF16` (device-dependent) |
| `candle_core::Result<T>` | Propagate via `.map_err(|e| e.to_string())` |
| `candle_nn::VarBuilder` | Load weights from safetensors/GGUF |
| `candle_nn::Module` | Trait bound for `model.forward(&xs)` |
| `candle_transformers::generation::LogitsProcessor` | Sampling strategy |
| `candle_transformers::models::phi3::Model` | Concrete model type (or Gemma/Qwen) |
| `candle_transformers::quantized_var_builder::VarBuilder` | GGUF quantized loader |

---

*End of audit.*