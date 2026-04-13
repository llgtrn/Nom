# Part 8: Vietnamese OSS Landscape for LLMs and NLP

**A practical scan of open-source Vietnamese LLM and NLP projects, cloned and analyzed for relevance to Novel.**

Research conducted: 2026-04-11
Local clone workspace: `.analysis/oss-vietnamese/`

> **Status 2026-04-14**: VN tokens are NOT adopted into Nom vocabulary (commit `ecd0609`). VN OSS projects listed here are retained as **CORPUS + EVAL** reference only. Any "tokenization into Nom" / "alias dictionaries from Vietnamese/Hán Việt/English layers" framing below is **superseded**. For the actual NL-intent surface see `nom-intent` M8 slice1 (commit `800baea`, 2026-04-14) — it does not depend on VN tokenizers. Adjacent work: M6 PyPI-100 corpus pilot is blocked on network access; VN corpus work similarly gated.

---

## Executive Summary

Yes, there is a real open-source ecosystem around Vietnamese AI and NLP.

It is not just one model or one toolkit. It already has:

- Vietnamese-first LLMs
- classic NLP pipelines
- tokenizers and text normalization tools
- speech and TTS projects
- evaluation harnesses
- curated corpora and benchmark collections

After searching and cloning representative projects, the clearest conclusion is:

**The Vietnamese OSS ecosystem is strongest today in three layers:**

1. **classical NLP tooling**  
   best represented by `underthesea` and `VnCoreNLP`
2. **released Vietnamese chat/base models**  
   best represented by `PhoGPT`, `ToRoLaMa`, Vistral, VinaLLaMA
3. **evaluation and resource aggregation**  
   best represented by `MELT` and `awsome-vietnamese-nlp`

For Novel specifically, the best immediate building blocks are:

- `underthesea` for Vietnamese preprocessing and normalization
- `MELT` for benchmarking and evaluation scaffolding
- `PhoGPT` as a Vietnamese-native LLM baseline/reference

The most important caution is:

many Vietnamese LLM repos are **release wrappers around model weights**, not full training stacks.

That is not bad, but it means the most reusable engineering value is often in:

- toolkits
- evaluation harnesses
- tokenizer/normalization pipelines

not necessarily in the chat-model repos themselves.

---

## 1. What I Cloned

Cloned locally:

- `VinAIResearch/PhoGPT`
- `undertheseanlp/underthesea`
- `allbyai/ToRoLaMa`
- `stair-lab/melt`
- `stair-lab/villm-eval` (older repo name/remote form of the MELT project)
- `vndee/awsome-vietnamese-nlp`

Looked up but did not fully analyze in depth:

- `vncorenlp/VnCoreNLP`
- `Viet-Mistral/Vistral-7B-Chat`
- `vilm/vinallama-*`
- `vilm-ai/vietcuna`
- `bkai-research/Vietnamese-LLaMA-2`

Reason for not going deeper on all of them:
some are primarily model weights on Hugging Face or thin release wrappers, so the highest-signal code analysis came from the five cloned repos above.

---

## 2. Repo-by-Repo Analysis

### 2.1 PhoGPT

Repo:
- https://github.com/VinAIResearch/PhoGPT
- local clone: `.analysis/oss-vietnamese/PhoGPT`

What it is:
- an official Vietnamese-first generative LLM release from VinAI Research
- monolingual base model plus chat variant

What the repo says:
- 3.7B parameter base/chat models
- 8192 context length
- Vietnamese corpus of 102B tokens
- chat tuning on 70K instruction pairs plus 290K conversations

What the cloned repo actually contains:
- only **5 tracked files**
- mostly:
  - README
  - license
  - fine-tuning YAML
  - sample instruction-following dataset

What this means:
- `PhoGPT` is primarily a **model release/integration repo**, not a full public training codebase
- it is still highly valuable as a **reference model family** and as evidence that Vietnamese-specific tokenizer/model design matters

Strengths:
- strong official Vietnamese model release
- explicit Vietnamese prompt templates
- documented inference paths for `transformers`, `llama.cpp`, `vLLM`, TGI
- clean baseline for Vietnamese-only generative tasks

Weaknesses:
- limited repo depth
- not a strong open engineering surface for end-to-end reproduction
- the README itself says it is weak on reasoning, coding, and mathematics

Relevance to Novel:
- useful as a **Vietnamese natural-language front-door baseline**
- useful to study Vietnamese-specific prompting and tokenizer assumptions
- **not** enough by itself to become Novel’s main research foundation

Verdict:
**important reference model, weak codebase substrate**

---

### 2.2 underthesea

Repo:
- https://github.com/undertheseanlp/underthesea
- local clone: `.analysis/oss-vietnamese/underthesea`

What it is:
- the most substantial general-purpose Vietnamese NLP toolkit I cloned

Local codebase signals:
- **1109 tracked files**
- active push history up to **2026-04-08**
- Python package with multiple submodules and optional extensions
- package version in `pyproject.toml`: `9.2.11`

What it supports:
- sentence segmentation
- text normalization
- address conversion
- word segmentation
- POS tagging
- chunking
- dependency parsing
- NER
- language detection
- TTS-related extras
- an agent layer using OpenAI/Azure OpenAI

Notable engineering detail:
- depends on `underthesea_core`, which points to a lower-level core implementation path
- optional extras for `deep`, `voice`, and `agent`

What the structure suggests:
- this is a **real maintained toolkit**, not only a paper artifact
- it has both library and CLI packaging
- it is expanding from classic NLP into broader developer ergonomics

Strengths:
- broad practical Vietnamese preprocessing surface
- mature packaging
- highly reusable for normalization and segmentation
- active maintenance
- useful dataset/resource management patterns

Weaknesses:
- the new “agent” functionality depends on external LLM APIs, so it is not a native Vietnamese OSS LLM in itself
- broad toolkit scope can make architecture less focused than a single-purpose library

Relevance to Novel:
- probably the **single most immediately useful OSS dependency reference**
- especially useful for:
  - Vietnamese normalization
  - tokenization/segmentation
  - preprocessing for natural-language-to-Nom resolution
  - building Vietnamese-friendly CLI/editor helpers

Verdict:
**best immediate OSS building block for Novel**

---

### 2.3 ToRoLaMa

Repo:
- https://github.com/allbyai/ToRoLaMa
- local clone: `.analysis/oss-vietnamese/ToRoLaMa`

What it is:
- a Vietnamese instruction-following/chat model release

Local codebase signals:
- **8 tracked files**
- last pushed in practice: **2024-01-04**
- lightweight repo with:
  - README
  - demo
  - inference script
  - requirements
  - disclaimer/license files

What the repo says:
- based on `Vietnamese-LLaMA2`
- trained with 430K high-quality multi-turn QA
- strong chat-style benchmark claims compared against PhoGPT and URA-LLaMA

What this means:
- like `PhoGPT`, this is more a **release shell** than a broad engineering platform
- still useful as a concrete example of Vietnamese chat-model packaging and serving

Strengths:
- simple to understand
- clear inference/deployment story
- Vietnamese chat formatting is explicit
- good example of FastChat-based deployment

Weaknesses:
- shallow repo
- older and apparently less active
- benchmark story is helpful but narrow

Relevance to Novel:
- useful as a **secondary chat-model reference**
- helpful if Novel wants to experiment with a lightweight Vietnamese conversational baseline
- less important than `PhoGPT` or `underthesea`

Verdict:
**good reference release, not a core platform**

---

### 2.4 MELT

Repo:
- https://github.com/stair-lab/melt
- local clone: `.analysis/oss-vietnamese/melt`

What it is:
- multilingual evaluation toolkit with serious Vietnamese support

Local codebase signals:
- **209 tracked files**
- push history through **2024-11-07**
- substantial `src/melt/` package with:
  - CLI
  - generation pipeline
  - dataset loaders
  - metric modules
  - wrappers for HF, VLLM, TGI, OpenAI, Gemini
  - evaluation pipelines
  - per-language config folders

What it supports for Vietnamese:
- summarization
- question answering
- sentiment analysis
- text classification
- toxicity detection
- open-ended knowledge
- multiple-choice knowledge
- translation
- reasoning
- math
- information retrieval

Notable design detail:
- uses `underthesea` as a dependency
- has language-specific config patterns instead of assuming English defaults

Strengths:
- strongest cloned repo for **evaluation rigor**
- very relevant if you want to compare Vietnamese LLMs fairly
- provides a task-and-metric architecture Novel could imitate later

Weaknesses:
- heavy dependency stack
- more evaluation-oriented than model-building-oriented
- broader multilingual framing means Vietnamese is one major target, not the entire system

Relevance to Novel:
- extremely useful if Novel gets an NL or LLM interface and you need to benchmark:
  - summarization
  - QA
  - instruction following
  - reasoning
  - retrieval
  - toxicity/bias

Verdict:
**best evaluation framework in this landscape for Novel**

---

### 2.5 awsome-vietnamese-nlp

Repo:
- https://github.com/vndee/awsome-vietnamese-nlp
- local clone: `.analysis/oss-vietnamese/awsome-vietnamese-nlp`

What it is:
- a curated list of Vietnamese NLP and LLM resources

Why it matters:
- not because it contains deep code
- because it gives a surprisingly good overview of the ecosystem:
  - LLMs
  - corpora
  - classical toolkits
  - pretrained models
  - speech resources
  - benchmarks

Strengths:
- very good discovery surface
- updated relatively recently
- useful for future repo scouting

Weaknesses:
- not a runtime dependency
- contains some outdated descriptions and mixed-quality links
- curation quality depends on maintainers

Relevance to Novel:
- excellent **map**, weak **engine**

Verdict:
**keep as an index, not as a foundation**

---

## 3. What I Learned About the Vietnamese OSS Ecosystem

### 3.1 The ecosystem is real, but fragmented

There is enough OSS to build real Vietnamese systems, but it is split across:

- GitHub repos
- Hugging Face model cards
- academic paper releases
- toolkit wrappers
- benchmark collections

So discovery is still harder than for English.

### 3.2 The strongest reusable engineering is still in classic NLP and eval

The most reusable code is not necessarily in chat-model repos.

The strongest codebases I found for reuse are:

- `underthesea`
- `MELT`
- `VnCoreNLP` as an older but still important reference

This matters for Novel because you need:

- Vietnamese normalization
- segmentation/tokenization
- NER/structure hints
- proper evaluation

more than you need another chat demo.

### 3.3 Many Vietnamese LLM repos are packaging layers around weights

That is normal for LLM OSS, but it changes how you should treat them.

Use them as:

- baselines
- inference references
- tokenizer/prompt-format references
- model comparison anchors

Do not assume they are broad public engineering platforms.

### 3.4 Evaluation is finally becoming serious

The presence of `MELT` and the ViLLM evaluation work is a strong sign of ecosystem maturity.

That means Vietnamese AI is moving beyond:

- "we fine-tuned a model"

toward:

- "we can compare models across task families with language-aware metrics"

This is exactly the direction Novel should respect.

---

## 4. Best OSS Choices For Novel

If the goal is to help Novel become a Vietnamese-first semantic programming system, here is the practical ranking.

### Best immediate use

1. **underthesea**
   - best for preprocessing, normalization, tokenization, and language-aware utilities

2. **MELT**
   - best for evaluation of any future LLM-facing or NL-facing Novel component

3. **PhoGPT**
   - best Vietnamese-first model baseline to compare against or prototype with

### Best ecosystem references

4. **VnCoreNLP**
   - important older reference for fast classical Vietnamese linguistic annotation

5. **awsome-vietnamese-nlp**
   - best discovery map for future exploration

### Lower priority for Novel

6. **ToRoLaMa**
   - useful as an existence proof, but less central than the three above

---

## 5. Concrete Ways Novel Could Use This OSS

### Path A: Vietnamese natural-language front end

Use:

- `underthesea` for normalization, segmentation, NER, sentence splitting
- `PhoGPT` or another Vietnamese LLM as the natural-language-to-concept resolver baseline
- `MELT` to evaluate resolution quality and task performance

### Path B: Vietnamese developer ergonomics

Use:

- `underthesea` to make the IDE/editor experience friendlier for Vietnamese text input
- alias dictionaries from Vietnamese/Hán Việt/English layers
- text normalization before Nom lookup

### Path C: Benchmark Novel’s Vietnamese understanding

Use:

- `MELT` tasks and datasets as an external benchmark harness
- compare Novel’s NL front end against:
  - PhoGPT
  - Vistral
  - VinaLLaMA
  - GPT-family baselines if needed

---

## 6. Risks And Gaps

### Gap 1: Reproducibility

Some model repos are light wrappers around released weights, so training reproducibility is partial.

### Gap 2: Licensing heterogeneity

Toolkits use permissive OSS licenses, but some LLMs inherit LLaMA-family or gated-model constraints.
That must be checked carefully before product use.

### Gap 3: Vietnamese-specific evaluation is still young

The ecosystem is improving, but benchmark fragmentation still exists.

### Gap 4: Coding/reasoning remains weaker than chat fluency

This is explicit even in model documentation like PhoGPT’s.
Vietnamese-native chat quality does not imply strong code or reasoning quality.

---

## 7. Bottom Line

If your question is:

> Is there any real OSS around Vietnamese LLMs or related AI/NLP?

The answer is:

**Yes, absolutely.**

If your question is:

> Which cloned projects matter most for Novel?

The answer is:

- **`underthesea`** for Vietnamese language tooling
- **`MELT`** for evaluation
- **`PhoGPT`** for Vietnamese-native LLM baseline

Those three together are the strongest practical starting point.

---

## 8. References

- PhoGPT: https://github.com/VinAIResearch/PhoGPT
- underthesea: https://github.com/undertheseanlp/underthesea
- ToRoLaMa: https://github.com/allbyai/ToRoLaMa
- MELT: https://github.com/stair-lab/melt
- VnCoreNLP: https://github.com/vncorenlp/VnCoreNLP
- awsome-vietnamese-nlp: https://github.com/vndee/awsome-vietnamese-nlp
- Vistral model card: https://huggingface.co/Viet-Mistral/Vistral-7B-Chat
- VinaLLaMA collection: https://huggingface.co/collections/vilm/vinallama-654a099308775ce78e630a6f
