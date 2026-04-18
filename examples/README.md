# Nom Examples — Golden Corpus (40 files)

Each file demonstrates the `define X that Y` natural syntax. The last statement is the result — no explicit return, no null, no panic by grammar.

## Files

### Core (10)

| File | Description |
|------|-------------|
| `hello.nomx` | Minimal greeting — defines a string and evaluates it |
| `add.nomx` | Integer addition — defines two numbers and sums them |
| `greet_name.nomx` | String concatenation — builds a greeting from a name binding |
| `fibonacci.nomx` | Sequence sketch — delegates to a named loop definition |
| `compose_image.nomx` | Image descriptor — binds width, height, and layer list into a summary string |
| `intent_resolve.nomx` | Intent resolution — maps a query to a kind with a confidence value |
| `skill_route.nomx` | Skill routing — maps an intent string to a named skill entry |
| `flow_record.nomx` | Flow recording — summarises step count and artifact output path |
| `bench_run.nomx` | Benchmark run — formats workload, platform, and timing into a result string |
| `dream_app.nomx` | Dream app report — emits name, score, and status for a scored app definition |

### Functional Patterns (5)

| File | Description |
|------|-------------|
| `map_list.nomx` | Apply a named transformation definition to each item in a list |
| `filter_list.nomx` | Keep items matching a named predicate definition |
| `reduce_list.nomx` | Fold a list to a single value using a binary accumulator definition |
| `compose_functions.nomx` | Chain two named definitions so output of first feeds input of second |
| `partial_apply.nomx` | Bind one argument of a two-argument definition, leaving the second deferred |

### Data Structures (4)

| File | Description |
|------|-------------|
| `build_record.nomx` | Construct a structured record binding name, kind, size, and timestamp |
| `nest_records.nomx` | Access a field of an inner record from an outer record definition |
| `update_field.nomx` | Produce an updated copy of a record with one field replaced |
| `merge_records.nomx` | Combine bindings from two separate record definitions into one result |

### Nom-Specific (6)

| File | Description |
|------|-------------|
| `kind_lookup.nomx` | Retrieve a kind entry by name from the dictionary, returning hash and description |
| `nomtu_search.nomx` | Search nomtu entries by text query, returning top-k matches with scores |
| `create_entry.nomx` | Create a new dictionary entry with word, kind, lifecycle state, and hash |
| `promote_entry.nomx` | Advance an entry from partial to complete lifecycle state |
| `graph_connect.nomx` | Wire two entry hashes with a typed directed edge of given weight |
| `run_benchmark.nomx` | Execute a named workload on a platform and record timing and custom counters |

### Media / Compose (5)

| File | Description |
|------|-------------|
| `resize_image.nomx` | Scale an image layer to target dimensions using a named filter |
| `mix_audio.nomx` | Blend two audio tracks at independent gain levels into a master output |
| `splice_video.nomx` | Join two video clips with a named transition over a frame count |
| `export_pdf.nomx` | Serialize a composition to PDF at given page count and DPI |
| `render_frame.nomx` | Produce a single output frame via a named render pipeline |

### Language Showcase (5)

| File | Description |
|------|-------------|
| `lazy_sequence.nomx` | Define a generator-backed sequence evaluated on demand |
| `tail_recursive.nomx` | Illustrate tail-call style using a delegating step definition |
| `monadic_chain.nomx` | Chain two definitions so each feeds the next in a linear pipeline |
| `pattern_variant.nomx` | Bind and describe two shape variants then combine results |
| `type_alias.nomx` | Define a named alias layered over an underlying primitive type |

### AI / Intent (5)

| File | Description |
|------|-------------|
| `classify_intent.nomx` | Map a natural language utterance to a predicted kind and top nomtu |
| `generate_response.nomx` | Produce an AI response from a prompt with configurable token and temperature settings |
| `embed_query.nomx` | Embed a text query into a fixed-dimension vector for similarity search |
| `react_loop.nomx` | Execute one observe-think-act cycle of the ReAct reasoning pattern |
| `tool_dispatch.nomx` | Invoke a named tool with structured arguments and surface the top result |

## How to Run

```
nom compose examples/hello.nomx
nom compose examples/add.nomx
nom compose examples/greet_name.nomx
nom compose examples/fibonacci.nomx
nom compose examples/compose_image.nomx
nom compose examples/intent_resolve.nomx
nom compose examples/skill_route.nomx
nom compose examples/flow_record.nomx
nom compose examples/bench_run.nomx
nom compose examples/dream_app.nomx
nom compose examples/map_list.nomx
nom compose examples/filter_list.nomx
nom compose examples/reduce_list.nomx
nom compose examples/compose_functions.nomx
nom compose examples/partial_apply.nomx
nom compose examples/build_record.nomx
nom compose examples/nest_records.nomx
nom compose examples/update_field.nomx
nom compose examples/merge_records.nomx
nom compose examples/kind_lookup.nomx
nom compose examples/nomtu_search.nomx
nom compose examples/create_entry.nomx
nom compose examples/promote_entry.nomx
nom compose examples/graph_connect.nomx
nom compose examples/run_benchmark.nomx
nom compose examples/resize_image.nomx
nom compose examples/mix_audio.nomx
nom compose examples/splice_video.nomx
nom compose examples/export_pdf.nomx
nom compose examples/render_frame.nomx
nom compose examples/lazy_sequence.nomx
nom compose examples/tail_recursive.nomx
nom compose examples/monadic_chain.nomx
nom compose examples/pattern_variant.nomx
nom compose examples/type_alias.nomx
nom compose examples/classify_intent.nomx
nom compose examples/generate_response.nomx
nom compose examples/embed_query.nomx
nom compose examples/react_loop.nomx
nom compose examples/tool_dispatch.nomx
```

Run all examples at once:

```
for f in examples/*.nomx; do nom compose "$f"; done
```
