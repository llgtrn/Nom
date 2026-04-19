import textwrap

content = textwrap.dedent(r'''
# LangChain Core Pattern Audit

**Date:** 2026-04-19  
**Source:** C:\Users\trngh\Documents\APP\Accelworld\upstreams\langchain-master\libs\core\langchain_core  
**Target Mapping:** 
om-canvas/crates/nom-compose/src/chain.rs  
**Analyst:** Pattern-Extraction Agent

---

## 1. Pattern Summary

LangChain's langchain_core is built around a **uniform Runnable protocol** that turns every component—prompts, models, tools, parsers—into a composable, streamable, batchable unit of work.

| Area | Core Pattern |
|------|-------------|
| **Runnable** | Abstract Runnable[Input, Output] trait exposing invoke/invoke, atch/batch, stream/stream, 	ransform/transform. Composition is declarative via the | operator (RunnableSequence) and dict literals (RunnableParallel). |
| **Tools** | BaseTool is a RunnableSerializable with an rgs_schema (Pydantic BaseModel or JSON-schema dict). The @tool decorator auto-infers schemas from type-hinted functions. Tools accept str \| dict \| ToolCall, validate inputs, and return ToolMessages. Injected args (InjectedToolCallId) are filtered from the LLM-facing schema. |
| **Prompts** | BasePromptTemplate is a RunnableSerializable[dict, PromptValue]. ChatPromptTemplate.from_messages() accepts tuples like ("system", "...") and MessagesPlaceholder for history. Supports f-string / jinja2 formatting and partial_variables. |
| **Output Parsers** | BaseOutputParser is a RunnableSerializable[LanguageModelOutput, T] with parse_result(result: list[Generation], *, partial: bool = False) -> T. PydanticOutputParser validates JSON against a Pydantic model; JsonOutputToolsParser / PydanticToolsParser extract OpenAI-style tool calls from AIMessage.tool_calls. BaseTransformOutputParser adds streaming support via chunk-wise 	ransform(). |

**Key Insight:** Everything is a Runnable. A chain is just prompt | model | parser, and because each stage implements the same protocol, sync/async/batch/stream behavior propagates automatically.

---

## 2. Key Source Files

### Runnables (Composition & Streaming)
- **unnables/base.py** (6,261 lines) — The monolithic core.
  - Runnable ABC: invoke, invoke, atch, batch, stream, stream, 	ransform, transform.
  - RunnableSequence (__or__ / | operator): sequential composition preserving streaming if all steps implement 	ransform.
  - RunnableParallel: concurrent map from the same input to named outputs.
  - RunnableLambda: wraps arbitrary callables; RunnableGenerator for streaming generators.
  - RunnableBinding: curries kwargs via ind(**kwargs).
  - Fluent modifiers: with_retry(), with_fallbacks(), with_config(), with_listeners(), with_types(), ssign(), pick().
- **unnables/schema.py** — StreamEvent, EventData TypedDicts for stream_events.
- **unnables/config.py** — RunnableConfig, ensure_config(), patch_config(), un_in_executor().

### Tools (Binding & Schemas)
- **	ools/base.py** (1,586 lines)
  - BaseTool (extends RunnableSerializable): 
ame, description, rgs_schema, 	ool_call_schema, invoke/invoke, _parse_input(), _run()/_arun().
  - SchemaAnnotationError, ToolException.
  - create_schema_from_function(): introspects Python signatures to build Pydantic models.
  - InjectedToolArg, InjectedToolCallId: runtime-injected params hidden from the LLM schema.
- **	ools/convert.py** — @tool decorator; overloads for Callable, Runnable, and string names.
- **	ools/simple.py** — Tool for single-string-input functions.
- **	ools/structured.py** — StructuredTool for multi-argument functions with explicit rgs_schema.

### Prompts (Templating)
- **prompts/chat.py** (1,491 lines)
  - ChatPromptTemplate (rom_messages(), __init__ accepting MessageLikeRepresentation).
  - MessagesPlaceholder: injects pre-built message lists; supports optional=True and 
_messages truncation.
  - HumanMessagePromptTemplate, AIMessagePromptTemplate, SystemMessagePromptTemplate.
- **prompts/base.py** — BasePromptTemplate: input_variables, partial_variables, output_parser, ormat().
- **prompts/string.py** — StringPromptTemplate, get_template_variables(), jinja2 formatter.

### Output Parsers (Structured Output)
- **output_parsers/base.py** (348 lines)
  - BaseLLMOutputParser: parse_result() / parse_result().
  - BaseOutputParser (extends RunnableSerializable): default invoke implementation wrapping parse_result over Generation / ChatGeneration.
- **output_parsers/pydantic.py** — PydanticOutputParser: parses JSON then calls model_validate() (v2) or parse_obj() (v1).
- **output_parsers/openai_tools.py** — JsonOutputToolsParser, JsonOutputKeyToolsParser, PydanticToolsParser: read AIMessage.tool_calls and dditional_kwargs["tool_calls"].
- **output_parsers/transform.py** — BaseTransformOutputParser: implements 	ransform() / transform() for chunk-wise parsing.
- **output_parsers/json.py** — JsonOutputParser, SimpleJsonOutputParser: partial-JSON safe parsing.

---

## 3. Nom Mapping

**Current Nom state:** 
om-canvas/crates/nom-compose/src/chain.rs (312 lines).

| LangChain Concept | Nom Equivalent (Today) | Gap / Opportunity |
|-------------------|------------------------|-------------------|
| Runnable[Input, Output] | Runnable trait: un(&self, input: &str, ctx: &ComposeContext) -> Result<String, String> | Nom is **untyped** (always &str -> String) and **sync-only**. Adding generics Runnable<I, O> and async (invoke) would mirror LangChain's flexibility. |
| RunnableSequence (\|) | Chain struct with Vec<Box<dyn Runnable>>; dd_step() builder; chain() method on trait. | Missing operator overloading (Rust's \| could be used). Missing streaming passthrough: LangChain only streams if every step implements 	ransform. Nom has no 	ransform at all. |
| RunnableParallel (dict concurrency) | **None** | New type needed: a Parallel struct executing named branches concurrently (e.g., 	okio::join!). |
| RunnableLambda | **None** | Nom runnables are all explicit structs. A FnRunnable<F> wrapper would let users inline closures. |
| ind(**kwargs) | **None** | Currying context into a runnable requires manual struct fields today. A BoundRunnable<R> wrapper would reduce boilerplate. |
| BaseTool / rgs_schema | DispatchRunnable routes by ctx.kind, but there is no schema introspection. | If Nom wants LLM-driven tool calling, it needs JSON-schema generation from Rust types (e.g., schemars) and a Tool trait with schema() -> Value. |
| @tool decorator | **None** | Rust macro #[nom_tool] on functions could auto-generate name/description/schema via syn + schemars. |
| ChatPromptTemplate.from_messages | **None** | Nom passes raw strings to LlmRunnable. A ChatPromptTemplate equivalent would let Nom construct message arrays (system/user/assistant) with placeholders before calling the LLM adapter. |
| MessagesPlaceholder | **None** | Conversation history injection is hard-coded or absent. A placeholder type would allow dynamic history insertion. |
| BaseOutputParser | **None** | LlmRunnable returns raw String. Wrapping it with a ParseRunnable<P> where P: OutputParser would let Nom validate JSON / tool calls / Pydantic-like structs. |
| PydanticOutputParser | **None** | Could map to a JsonSchemaParser<T: Deserialize> using serde_json. |
| JsonOutputToolsParser | **None** | If Nom adds tool-calling support, this parser extracts 	ool_calls arrays from model responses. |
| with_retry() / with_fallbacks() | **None** | Resilience wrappers around Runnable could be added generically in Rust (e.g., RetryRunnable<R>). |

**Recommended adoption priority for Nom:**
1. **Typed Runnable<I, O>** — unlocks compile-time pipeline validation.
2. **FnRunnable / ind** — lowers barrier for ad-hoc transformations.
3. **Tool trait + schema generation** — prerequisite for agentic tool use.
4. **ChatPromptTemplate + MessagesPlaceholder** — needed for multi-turn LLM interactions.
5. **OutputParser trait** — enables structured output without manual regex in every chain.
6. **Async / batch / parallel** — scaling concerns; can be deferred if Nom is currently single-user.

---

## 4. Licensing / Complexity Notes

### Licensing
- LangChain is released under the **MIT License** (langchain-master/LICENSE).
- Safe to study, adapt, and port patterns into Nom without copyleft concerns. Attribution required in derivative code or docs if substantial logic is translated.

### Complexity Warnings
- **unnables/base.py is 6,261 lines** (~222 KB). The whole unnables package is ~468 KB of Python. This is not a thin abstraction; it handles Pydantic v1/v2 dual compatibility, async executor bridging, deep callback/tracer integration, and extensive edge-case handling.
- **Pydantic dependency** — LangChain leans heavily on runtime schema introspection. A Rust port cannot directly replicate @tool decorator magic without procedural macros (schemars, serde, syn).
- **Streaming logic is subtle** — 	ransform() must correctly handle back-pressure, partial chunks, and cumulative state (see BaseCumulativeTransformOutputParser). Nom currently has no iterator-based streaming abstraction.
- **Tool injection** — InjectedToolCallId and InjectedToolArg require careful schema subsetting so the LLM never sees injected fields. LangChain does this via _create_subset_model() and annotation scanning.
- **Error handling** — LangChain distinguishes ToolException, OutputParserException, ValidationError, and retryable vs. non-retryable failures. Nom uses plain Result<String, String>; richer error types would be needed for parity.

---

## 5. Adoption Effort Estimate

| Feature | Effort | Notes |
|---------|--------|-------|
| **Generic Runnable<I, O>** | Small | Refactor Runnable to take generic input/output types; update Chain to propagate them. |
| **Operator overloading (\|)** | Tiny | Implement BitOr for Box<dyn Runnable> or a wrapper type. |
| **FnRunnable wrapper** | Small | Box a closure; trait-object safety limits generics, so String-based IO likely remains easiest. |
| **ind() / currying** | Small | A BoundRunnable struct storing Box<dyn Runnable> plus a HashMap<String, String> of pre-bound args. |
| **Tool trait + JSON schema** | Medium | Add schemars dependency; derive JsonSchema on tool-input structs. Build a Tool trait with 
ame(), description(), schema(), invoke(). |
| **@tool-like macro** | Medium-Large | Procedural macro crate to introspect function signatures and generate Tool impl + schemars schema at compile time. |
| **ChatPromptTemplate** | Medium | Message enum (System, User, Assistant, Placeholder); template interpolation (f-string or Handlebars); rom_messages constructor. |
| **MessagesPlaceholder** | Small | Add a Placeholder variant to the message enum; resolve at invoke time. |
| **OutputParser trait** | Small | Trait with parse(&str) -> Result<T, ParseError>; wrap LlmRunnable with a parser step. |
| **PydanticOutputParser equivalent** | Small | Use serde_json::from_str::<T>() where T: DeserializeOwned; schema from schemars. |
| **JsonOutputToolsParser** | Medium | Parse OpenAI-style tool-call JSON; map to Nom tool invocations. Requires tool-call message format in Nom. |
| **Streaming (	ransform)** | Medium-Large | Need async iterators (Stream / AsyncIterator equivalents); update every runnable to yield chunks. LlmRunnable must support SSE or chunked responses. |
| **Parallel execution (RunnableParallel)** | Medium | 	okio::join! or utures::future::join_all over a HashMap<String, Box<dyn Runnable>>. |
| **Batching (atch/batch)** | Medium | Input fan-out with concurrency limits; output fan-in preserving order. |
| **Retry / Fallbacks** | Small-Medium | Generic wrappers using 	okio-retry or manual loop logic. |
| **Tracing / Callbacks** | Large | LangChain's callback system (CallbackManager, RunManager) is extensive. Nom would need its own telemetry abstraction if desired. |

### Bottom Line
- **Minimum viable adoption** (typed Runnable, basic Tool trait, prompt template, output parser): **~1–2 weeks** for a Rust developer familiar with macros and serde.
- **Full feature parity** (async, streaming, batching, tracing, full schema inference): **~2–3 months** of dedicated work.
- **Recommendation:** Adopt the **protocol shape** (uniform Runnable, Chain as sequence, Tool as schema-bearing runnable) immediately. Defer streaming, batching, and tracing until Nom's LLM use cases demand them.
''').lstrip()

with open(r'C:\Users\trngh\Documents\GitHub\Nom\.archive\audits\2026-04-19-langchain-pattern.md', 'w', encoding='utf-8') as f:
    f.write(content)
