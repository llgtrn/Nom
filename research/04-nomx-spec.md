# 04 — `.nomx` Source Format Spec

`.nomx` is the single Nom source format. Two earlier surface variants (the
prose form and the typed-slot form) are scheduled to merge into one
canonical surface combining prose readability with typed-slot precision.
Until the merge ships, both are accepted by the parser; afterwards, only
the merged form is accepted and the legacy parser path is deleted.

## Top-level decl shape

Every top-level decl opens with the determiner `the` followed by a kind
noun and a name:

```nomx
the function <name> is
  intended to <prose>.
  uses the @<Kind> matching "<intent prose>" with at-least <0..1> confidence.
  requires <prose-precondition>.
  ensures <prose-postcondition>.
  hazard <prose-hazard-note>.
  favor <quality_name>.
```

Closed kind nouns: `function`, `module`, `concept`, `screen`, `data`, `event`,
`media`, `property`, `scenario`.

## Clause vocabulary

`intended` — every decl carries one. Followed by `to` plus a single prose
sentence ending with a period.

`uses` — typed-slot reference into the dictionary. Pattern: `uses the @Kind
matching "<quoted intent prose>" with at-least <0..1 confidence threshold>
confidence.` The resolver returns the highest-ranked dictionary entity
exceeding the threshold; below threshold, the build fails.

`requires` — preconditions on inputs. Plain prose; the build stage attempts
to translate to a check expression for the parser-acceptable subset.

`ensures` — postconditions on outputs. Plain prose with the W49 quantifier
vocabulary (`every`, `no`, `some`, `at-least N`, `at-most N`, `exactly N`)
recognized for machine-checkable patterns.

`hazard` — known pitfalls. Plain prose. Surfaced in build reports; some
hazards have machine-check counterparts that the build stage exercises.

`favor` — quality-axis declaration. Followed by a `quality_name` row from
`grammar.sqlite.quality_names`. Multiple `favor` clauses allowed; MECE
validator enforces required-axis coverage at the concept layer.

`exposes` — only on `data` and `screen` decls. Pattern: `exposes <field>
[at tag <int>] as <type> [with payload <field-list>].` Tagged-variant
support via `at tag` for sum types.

`generator` — only on `property` decls. Pattern: `generator <prose-domain
descriptor>.` Bounds the universe over which the property is asserted.

`composes` — only on `module` and `concept` decls. Pattern: `composes
<entity-ref> [then <entity-ref>]+.` Linear pipeline; the build stage may
fuse, reorder, or specialize when steps are pure.

`given`, `when`, `then` — only on `scenario` decls. Each clause is a single
prose sentence. The triple constitutes one asserted behavior.

## Quantifier vocabulary

`every`, `no`, `some`, `at-least N`, `at-most N`, `exactly N` — recognized
inside `requires` / `ensures` / `hazard` clauses. The build-stage checker
identifies these as quantified shapes and emits machine-checkable
obligations where possible.

## Typed-slot kind markers

`@Function`, `@Module`, `@Concept`, `@Screen`, `@Data`, `@Event`, `@Media`,
`@Property`, `@Scenario`, `@Composition`, `@Route`. The `@Route` marker
covers HTTP method+path / event-handler / CLI-subcommand styles via the
existing typed-slot resolver.

## Lock semantics

After first build, every prose `matching "<intent>"` clause is rewritten to
`name@hash` so subsequent builds are reproducible. The source file is the
lock; no sidecar lockfile.

## Strictness

A strict-mode validator flags typed-slot refs missing the `with at-least N
confidence` clause. The intent threshold is mandatory in strict builds; the
default parser still accepts the loose form for authoring drafts.
