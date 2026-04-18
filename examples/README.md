# Nom Examples — B2 Golden Corpus

Each file demonstrates the `define X that Y` natural syntax. The last statement is the result — no explicit return, no null, no panic by grammar.

## Files

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
```

Run all examples at once:

```
for f in examples/*.nomx; do nom compose "$f"; done
```
