-- Nom grammar baseline — version 1.0
--
-- Canonical seed rows for grammar.sqlite. Run via `nom grammar import
-- crates/nom-grammar/data/baseline.sql` against an initialized DB.
-- Idempotent: every INSERT uses INSERT OR IGNORE so re-imports don't
-- duplicate or fail. Per the no-Rust-bundled-data rule, every grammar
-- fact lives here as data, never as a const array in Rust.
--
-- Foreign-language programming-language names are absent by invariant.
-- Each row's source_ref points to a Nom design doc in research/; never
-- to an external language.

BEGIN;

-- ── Kinds (the closed 9-noun set) ───────────────────────────────────

INSERT OR IGNORE INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) VALUES
('function', 'Named computation with input types + output type + requires/ensures/hazard contract clauses.', '[]', '[]', 'baseline-1.0', NULL),
('module', 'Tier-1 composition: several entities grouped with optional composition expressions.', '[]', '[]', 'baseline-1.0', NULL),
('concept', 'Tier-2 big-scope container: one or more concepts with dictionary-relative index over the entity store.', '[]', '[]', 'baseline-1.0', NULL),
('screen', 'User-facing UI surface or rendered artifact or architectural diagram.', '[]', '[]', 'baseline-1.0', NULL),
('data', 'Structural type or tagged variant. Holds exposes field declarations with payload types.', '[]', '[]', 'baseline-1.0', NULL),
('event', 'Named event signal. W49-quantified ensures clauses describe delivery semantics.', '[]', '[]', 'baseline-1.0', NULL),
('media', 'Image, audio, video, three-dimensional, or typography artifact composable via the same operators as functions.', '[]', '[]', 'baseline-1.0', NULL),
('property', 'Universally-quantified claim over a generator. Wedge W41.', '[]', '[]', 'baseline-1.0', NULL),
('scenario', 'Asserted-behavior claim with given/when/then triple. Wedge W46.', '[]', '[]', 'baseline-1.0', NULL);

-- ── Quality names (the 10 founding axes per MEMORY.md) ──────────────

INSERT OR IGNORE INTO quality_names (name, axis, metric_function, cardinality, required_at, source_ref, notes) VALUES
('forward_compatibility', 'semver_api',         NULL, 'any',                  NULL,  'doc 08', NULL),
('numerical_stability',   'numeric',            NULL, 'any',                  NULL,  'doc 08', NULL),
('gas_efficiency',        'on_chain_cost',      NULL, 'any',                  NULL,  'doc 08', NULL),
('synthesizability',      'hardware',           NULL, 'any',                  NULL,  'doc 08', NULL),
('minimum_cost',          'optimization',       NULL, 'any',                  NULL,  'doc 08', NULL),
('statistical_rigor',     'stats',              NULL, 'any',                  NULL,  'doc 08', NULL),
('availability',          'ops',                NULL, 'exactly_one_per_app',  'app', 'doc 08', NULL),
('auditability',          'ops',                NULL, 'any',                  NULL,  'doc 08', NULL),
('accessibility',         'ops',                NULL, 'exactly_one_per_app',  'app', 'doc 08', NULL),
('totality',              'proofs',             NULL, 'any',                  NULL,  'doc 08', NULL),
('correctness',           'semantics',          NULL, 'any',                  NULL,  'doc 08', NULL),
('determinism',           'semantics',          NULL, 'any',                  NULL,  'doc 08', NULL),
('clarity',               'authoring',          NULL, 'any',                  NULL,  'doc 08', NULL),
('documentation',         'authoring',          NULL, 'any',                  NULL,  'doc 08', NULL),
('discoverability',       'authoring',          NULL, 'any',                  NULL,  'doc 08', NULL),
('reproducibility',       'ops',                NULL, 'any',                  NULL,  'doc 08', NULL),
('portability',           'ops',                NULL, 'any',                  NULL,  'doc 08', NULL),
('responsiveness',        'performance',        NULL, 'any',                  NULL,  'doc 08', NULL),
('latency',               'performance',        NULL, 'any',                  NULL,  'doc 08', NULL),
('performance',           'performance',        NULL, 'any',                  NULL,  'doc 08', NULL);

-- ── Keywords (reserved tokens recognized by the lexer) ──────────────

-- Determiner
INSERT OR IGNORE INTO keywords (token, role,             kind_scope, source_ref, shipped_commit, notes) VALUES
('the',   'determiner',     NULL,       'doc 04',   'baseline-1.0', 'every top-level decl opens with `the`');

-- Kind nouns (the 9 closed kinds appear as Tok::Kind in the lexer)
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('function', 'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('module',   'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('concept',  'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('screen',   'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('data',     'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('event',    'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('media',    'kind_noun', NULL, 'doc 04', 'baseline-1.0', NULL),
('property', 'kind_noun', NULL, 'W41',    'baseline-1.0', NULL),
('scenario', 'kind_noun', NULL, 'W46',    'baseline-1.0', NULL);

-- Kind markers (typed-slot @Kind syntax per doc 04)
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('@Function',    'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Module',      'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Concept',     'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Screen',      'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Data',        'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Event',       'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Media',       'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL),
('@Property',    'kind_marker', NULL, 'W41',    'baseline-1.0', NULL),
('@Scenario',    'kind_marker', NULL, 'W46',    'baseline-1.0', NULL),
('@Composition', 'kind_marker', NULL, 'doc 04', 'baseline-1.0', NULL);

-- Clause openers
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('intended',  'clause_opener', NULL, 'doc 04', 'baseline-1.0', 'pairs with `to` to introduce intent prose'),
('requires',  'clause_opener', '["function","property","concept","scenario"]', 'doc 04', 'baseline-1.0', NULL),
('ensures',   'clause_opener', '["function","property","concept","scenario","event"]', 'doc 04', 'baseline-1.0', NULL),
('hazard',    'clause_opener', '["function","concept","property","event"]', 'doc 04', 'baseline-1.0', NULL),
('uses',      'clause_opener', '["function","concept","property","scenario","module","screen","media","event"]', 'doc 04', 'baseline-1.0', 'introduces typed-slot @Kind refs'),
('exposes',   'clause_opener', '["data","screen","event"]', 'doc 04', 'baseline-1.0', NULL),
('favor',     'clause_opener', NULL, 'doc 04', 'baseline-1.0', 'pairs with a quality_names row'),
('generator', 'clause_opener', '["property"]', 'W42', 'baseline-1.0', NULL),
('composes',  'clause_opener', '["module","concept"]', 'doc 04', 'baseline-1.0', NULL),
('given',     'clause_opener', '["scenario"]', 'W47', 'baseline-1.0', NULL),
('when',      'clause_opener', '["scenario"]', 'W47', 'baseline-1.0', NULL),
('then',      'clause_opener', '["scenario"]', 'W47', 'baseline-1.0', NULL),
('benefit',   'clause_opener', '["function","property","event"]', 'doc 04', 'baseline-1.0', 'positive effect valence');

-- Ref-slot vocabulary
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('matching',   'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('with',       'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('at-least',   'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('confidence', 'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL);

-- Connectives
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('is',  'connective', NULL, 'doc 04', 'baseline-1.0', 'copula after decl opener'),
('to',  'connective', NULL, 'doc 04', 'baseline-1.0', 'pairs with intended'),
('as',  'connective', NULL, 'doc 04', 'baseline-1.0', 'exposes X as Y'),
('of',  'connective', NULL, 'doc 04', 'baseline-1.0', NULL),
('and', 'connective', NULL, 'doc 04', 'baseline-1.0', NULL),
('or',  'connective', NULL, 'doc 04', 'baseline-1.0', NULL);

-- Keyword synonyms (S1 rewrites these to their canonical form).
-- Corpus-driven: authors writing `proof` or `composition` want the
-- `property` / `module` kinds; rather than extend the closed 9-kind
-- set, we rewrite at lex time so the archive captures stay
-- canonical-form-free while still parsing.
INSERT OR IGNORE INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) VALUES
('proof',        'property', 'baseline-1.0', 'baseline-1.0', 'universally-quantified theorem claim maps to property kind'),
('composition',  'module',   'baseline-1.0', 'baseline-1.0', 'composition-of-functions idiom maps to module kind'),
('row',          'data',     'baseline-1.0', 'baseline-1.0', 'data-table row idiom maps to data kind'),
('diagram',      'screen',   'baseline-1.0', 'baseline-1.0', 'architecture diagram — screen is the generalized rendered-artifact kind'),
('participants', 'data',     'baseline-1.0', 'baseline-1.0', 'workflow participant list maps to data kind'),
('layout',       'screen',   'baseline-1.0', 'baseline-1.0', 'UI layout arrangement maps to screen kind'),
('format',       'data',     'baseline-1.0', 'baseline-1.0', 'data format specification maps to data kind');

-- Clause shapes (per-kind grammar)
-- function (6 clauses): intended (req) / uses / requires / ensures (≥1 req) / hazard / favor
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('function', 'intended', 1, 1, '''intended to'' <prose-sentence> ''.''', 'doc 04'),
('function', 'uses',     0, 2, '''uses the'' ''@'' Kind ''matching'' <quoted-prose> ''with at-least'' <0..1> ''confidence'' ''.''', 'doc 04'),
('function', 'requires', 0, 3, '''requires'' <prose> ''.''', 'doc 04'),
('function', 'ensures',  2, 4, '''ensures'' <prose> ''.''', 'doc 04'),
('function', 'hazard',   0, 5, '''hazard'' <prose> ''.''', 'doc 04'),
('function', 'favor',    0, 6, '''favor'' <quality-name> ''.''', 'doc 08');

-- data (3 clauses): intended (req) / exposes (≥1 req) / favor
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('data', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('data', 'exposes',  2, 2, '''exposes'' <field> (''at tag'' <int>)? ''as'' <type> ''.''', 'doc 04'),
('data', 'favor',    0, 3, '''favor'' <quality-name> ''.''', 'doc 08');

-- concept (8 clauses)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('concept', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('concept', 'uses',     0, 2, '''uses the'' ''@'' Kind ''matching'' <quoted-prose> ...', 'doc 04'),
('concept', 'composes', 0, 3, '''composes'' <ref> (''then'' <ref>)* ''.''', 'doc 04'),
('concept', 'requires', 0, 4, '''requires'' <prose> ''.''', 'doc 04'),
('concept', 'ensures',  0, 5, '''ensures'' <prose> ''.''', 'doc 04'),
('concept', 'hazard',   0, 6, '''hazard'' <prose> ''.''', 'doc 04'),
('concept', 'exposes',  0, 7, '''exposes'' <name-list> ''.''', 'doc 04'),
('concept', 'favor',    0, 8, '''favor'' <quality-name> ''.''', 'doc 08');

-- module (4 clauses)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('module', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('module', 'uses',     0, 2, '''uses the'' ''@'' Kind ''matching'' ...', 'doc 04'),
('module', 'composes', 0, 3, '''composes'' <ref> (''then'' <ref>)* ''.''', 'doc 04'),
('module', 'favor',    0, 4, '''favor'' <quality-name> ''.''', 'doc 08');

-- property (6 clauses; W41 + W42 generator)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('property', 'intended',  1, 1, '''intended to assert'' <claim> ''.''', 'W41'),
('property', 'generator', 1, 2, '''generator'' <prose-domain> ''.''', 'W42'),
('property', 'uses',      0, 3, '''uses the'' ''@'' Kind ''matching'' ...', 'doc 04'),
('property', 'requires',  0, 4, '''requires'' <prose> ''.''', 'doc 04'),
('property', 'ensures',   2, 5, '''ensures'' <universal-claim> ''.''', 'W41'),
('property', 'favor',     0, 6, '''favor'' <quality-name> ''.''', 'doc 08');

-- scenario (5 clauses; W46/W47)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('scenario', 'intended', 1, 1, '''intended to describe'' <prose> ''.''', 'W46'),
('scenario', 'given',    1, 2, '''given'' <prose> ''.''', 'W47'),
('scenario', 'when',     1, 3, '''when'' <prose> ''.''', 'W47'),
('scenario', 'then',     1, 4, '''then'' <prose> ''.''', 'W47'),
('scenario', 'favor',    0, 5, '''favor'' <quality-name> ''.''', 'doc 08');

-- screen (4 clauses)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('screen', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('screen', 'uses',     0, 2, '''uses the'' ''@'' Kind ''matching'' ...', 'doc 04'),
('screen', 'exposes',  0, 3, '''exposes'' <field> ''as'' <type> ''.''', 'doc 04'),
('screen', 'favor',    0, 4, '''favor'' <quality-name> ''.''', 'doc 08');

-- event (4 clauses)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('event', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('event', 'exposes',  0, 2, '''exposes'' <field> ''as'' <type> ''.''', 'doc 04'),
('event', 'ensures',  0, 3, '''ensures'' <quantified-delivery-claim> ''.''', 'doc 04'),
('event', 'favor',    0, 4, '''favor'' <quality-name> ''.''', 'doc 08');

-- media (3 clauses)
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('media', 'intended', 1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('media', 'uses',     0, 2, '''uses the'' ''@'' Kind ''matching'' ...', 'doc 04'),
('media', 'favor',    0, 3, '''favor'' <quality-name> ''.''', 'doc 08');

-- ── Patterns (canonical authoring shapes) ──
--
-- Each row is a reusable authoring shape an AI client consults to
-- find the canonical rendering for a given intent. Seed is minimal;
-- additional patterns are user-added via `nom grammar add-pattern`
-- or future batch imports.

INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'pure-function-contract',
  'a named computation with contract clauses and quality favors',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to <prose>.\n  uses the @Function matching "<intent>" with at-least 0.9 confidence.\n  requires <pre>.\n  ensures <post>.\n  hazard <risk>.\n  favor correctness.',
  '[]',
  '["correctness"]',
  '[]'
),
(
  'exposes-data-shape',
  'a tagged record with named fields and their types',
  '["data"]',
  '["intended","exposes","favor"]',
  '[]',
  'the data <Name> is\n  intended to <prose>.\n  exposes <field> as <type>.\n  exposes <field> as <type>.\n  favor documentation.',
  '[]',
  '["documentation","clarity"]',
  '[]'
),
(
  'concept-composition',
  'a bundled concept composing supporting functions and data via typed-slot references',
  '["concept","function","data"]',
  '["intended","uses","composes","exposes","favor"]',
  '["@Function","@Data","@Screen"]',
  'the concept <name> is\n  intended to <prose>.\n  uses the @Function matching "<intent>" with at-least 0.9 confidence.\n  composes <ref> then <ref>.\n  exposes <surface>.\n  favor correctness.',
  '[]',
  '["correctness","availability"]',
  '[]'
),
(
  'property-quantified-claim',
  'a universally-quantified claim over a generator; the substrate for theorem-proving and property-based tests',
  '["property"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Function"]',
  'the property <name> is\n  intended to assert <prose-claim>.\n  generator <prose-domain>.\n  uses the @Function matching "<peer-lemma>" with at-least 0.9 confidence.\n  requires <domain-precondition>.\n  ensures for every <x> in the generator, <claim-about-x>.\n  favor correctness.',
  '[]',
  '["correctness","totality"]',
  '[]'
),
(
  'scenario-given-when-then',
  'a behavior-driven scenario asserting a specific observable outcome under a specific setup',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '[]',
  'the scenario <name> is\n  intended to describe <prose>.\n  given <prose-setup>.\n  when <prose-action>.\n  then <prose-outcome>.\n  favor correctness.',
  '[]',
  '["correctness","clarity"]',
  '[]'
),
(
  'event-quantified-delivery',
  'a named event signal with quantified delivery semantics (at-least-once, at-most-once, exactly-once)',
  '["event"]',
  '["intended","exposes","ensures","favor"]',
  '[]',
  'the event <name> is\n  intended to <prose>.\n  exposes <field> as <type>.\n  ensures delivery is <quantifier> to every subscriber.\n  favor availability.',
  '["duplicate delivery when quantifier is at-least-once"]',
  '["availability","auditability"]',
  '[]'
),
(
  'screen-exposes-surface',
  'a rendered user-facing surface — UI screen, diagram, typeset document — with typed slots for its content',
  '["screen"]',
  '["intended","uses","exposes","favor"]',
  '["@Data","@Function"]',
  'the screen <name> is\n  intended to <prose>.\n  uses the @Data matching "<model>" with at-least 0.9 confidence.\n  exposes <surface-field> as <type>.\n  favor accessibility.',
  '[]',
  '["accessibility","responsiveness"]',
  '[]'
),
(
  'supervised-process-tree',
  'a fault-tolerant concept composing supervised children with restart policies',
  '["concept","data","function"]',
  '["intended","uses","composes","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <supervisor> is\n  intended to supervise a set of child processes with lifecycle policies.\n  uses the @Data matching "child spec" with at-least 0.9 confidence.\n  composes <child-a> then <child-b>.\n  hazard a supervisor cascade can restart otherwise-healthy children.\n  favor availability.',
  '["restart storms","hidden cascade failures"]',
  '["availability","auditability"]',
  '[]'
),
(
  'tagged-variant-errors',
  'errors as named tagged variants of a shared data kind, each with domain-specific payload',
  '["data","function"]',
  '["intended","exposes","hazard","favor"]',
  '[]',
  'the data <Outcome> is\n  intended to represent either a success payload or one of a fixed set of named failure variants.\n  exposes variant as one of: success, not_found, conflict, forbidden.\n  exposes payload as text.\n  favor clarity.',
  '["callers who ignore a variant silently skip error handling"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'retry-policy',
  'a function-level orchestrator clause describing retry, backoff, and giveup semantics under transient failure',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the function <name> is\n  intended to perform a potentially-transient operation with bounded retry.\n  uses the @Function matching "one attempt" with at-least 0.9 confidence.\n  requires the underlying operation is idempotent.\n  ensures at most N attempts are made with exponential backoff between tries.\n  hazard retrying a non-idempotent side-effect can double-apply it.\n  favor availability.',
  '["non-idempotent side-effects","thundering-herd on coordinated retry"]',
  '["availability","auditability"]',
  '[]'
),
(
  'effect-handler',
  'a captured effect with a handler that determines the effect interpretation; the substrate for generators, async, non-determinism',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <handler> is\n  intended to interpret a captured effect into a concrete action.\n  uses the @Data matching "effect token" with at-least 0.9 confidence.\n  requires the handler is total over the effect''s variant set.\n  ensures every raised effect resumes with a well-typed continuation value.\n  hazard a partial handler silently discards unhandled effect variants.\n  favor correctness.',
  '["partial handlers","handler-loop infinite resumption"]',
  '["correctness","totality"]',
  '[]'
),
(
  'reactive-ui-state-machine',
  'a user-interface concept describing states, transitions, and guards without hidden mutation',
  '["concept","data","event","screen"]',
  '["intended","uses","composes","ensures","favor"]',
  '["@Data","@Event","@Screen"]',
  'the concept <ui-machine> is\n  intended to describe the finite-state behavior of a user-interface surface.\n  uses the @Data matching "machine state" with at-least 0.9 confidence.\n  uses the @Event matching "trigger" with at-least 0.9 confidence.\n  composes <state-a> then <state-b>.\n  ensures every transition is guarded by a declared predicate.\n  favor responsiveness.',
  '["unreachable states","transition guards that depend on external mutable state"]',
  '["responsiveness","accessibility"]',
  '[]'
),
(
  'content-addressed-build',
  'a build function whose output is hashed over its inputs — pinned source + pinned dependency closure + pinned commands — yielding reproducibility by construction',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <build> is\n  intended to produce a reproducible, content-addressed build artifact.\n  uses the @Data matching "pinned source" with at-least 0.95 confidence.\n  uses the @Data matching "pinned inputs" with at-least 0.95 confidence.\n  requires every input is pinned to a specific hash.\n  ensures identical inputs produce a byte-identical output artifact.\n  hazard any unpinned system dependency leaks ambient state.\n  favor reproducibility.',
  '["ambient state leaks","impure build steps (network, clock, random)"]',
  '["reproducibility","correctness"]',
  '[]'
),
(
  'schema-query',
  'a declarative query over a structured data store returning a typed projection of matching rows',
  '["function","data"]',
  '["intended","uses","requires","ensures","favor"]',
  '["@Data"]',
  'the function <query> is\n  intended to return every row matching the declared predicates, projected onto a typed result shape.\n  uses the @Data matching "source collection" with at-least 0.9 confidence.\n  requires the predicates are total over the row schema.\n  ensures the result is stable under repeated identical queries.\n  favor determinism.',
  '["N+1 access patterns","implicit collection scans"]',
  '["determinism","auditability"]',
  '[]'
),
(
  'pipeline-transformation',
  'a chain of pure transformations over a stream of input records, yielding an output stream of derived records',
  '["function","data"]',
  '["intended","uses","composes","ensures","favor"]',
  '["@Function","@Data"]',
  'the function <pipeline> is\n  intended to map every input record through a fixed sequence of pure transformations.\n  uses the @Function matching "stage" with at-least 0.9 confidence.\n  composes <filter> then <map> then <aggregate>.\n  ensures output order matches input order when the pipeline is stateless.\n  favor clarity.',
  '["stateful stages hidden inside a seemingly-pure pipeline"]',
  '["clarity","determinism"]',
  '[]'
),
(
  'network-api-endpoint',
  'a named HTTP or RPC endpoint with typed request and response shapes and explicit failure modes',
  '["function","data","event"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <endpoint> is\n  intended to accept a typed request and return a typed response or a named error.\n  uses the @Data matching "request shape" with at-least 0.9 confidence.\n  uses the @Data matching "response shape" with at-least 0.9 confidence.\n  requires the caller is authenticated.\n  ensures the response includes a correlation identifier.\n  hazard unbounded request bodies exhaust server memory.\n  favor availability.',
  '["missing rate limits","unbounded request bodies"]',
  '["availability","auditability"]',
  '[]'
),
(
  'verified-imperative',
  'imperative code carrying contracts that are checked at authoring time rather than deferred to runtime',
  '["function"]',
  '["intended","requires","ensures","hazard","favor"]',
  '[]',
  'the function <step> is\n  intended to perform a bounded imperative step with authored-in contract verification.\n  requires the precondition is total over the input type.\n  ensures the postcondition holds on every accepted input.\n  hazard loops without a declared variant do not terminate.\n  favor totality.',
  '["unverified loops","side-effects not captured by the contract"]',
  '["totality","correctness"]',
  '[]'
),
(
  'algebraic-law',
  'a named law over an operation — associativity, commutativity, identity, distributivity — checked as a universally-quantified claim',
  '["property","function"]',
  '["intended","generator","uses","ensures","favor"]',
  '["@Function"]',
  'the property <law> is\n  intended to assert that <operation> satisfies the declared algebraic law over its input domain.\n  generator pairs of inputs drawn from the operation''s declared domain.\n  uses the @Function matching "<operation>" with at-least 0.95 confidence.\n  ensures for every generated input, the law holds as an equality.\n  favor correctness.',
  '["partial operations whose law only holds on a subdomain"]',
  '["correctness","totality"]',
  '[]'
),
(
  'monadic-do-sequence',
  'a sequential composition of effectful steps where each step may fail or produce a value consumed by the next',
  '["function"]',
  '["intended","uses","composes","requires","ensures","favor"]',
  '["@Function"]',
  'the function <sequence> is\n  intended to perform a sequence of effectful steps where each step depends on the previous step''s value.\n  uses the @Function matching "lift step" with at-least 0.9 confidence.\n  composes <step-a> then <step-b> then <step-c>.\n  requires every step is total over its input type.\n  ensures a failure at any step short-circuits the remainder.\n  favor clarity.',
  '["a step that forgets to propagate its failure breaks the short-circuit guarantee"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'first-class-continuation',
  'a function that captures its own remaining-computation as a first-class value that may be resumed later, enabling coroutines, non-determinism, or early exit',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <capture> is\n  intended to snapshot the current continuation as a first-class value the caller may invoke later.\n  uses the @Data matching "continuation token" with at-least 0.9 confidence.\n  requires the captured continuation is resumed at most once.\n  ensures resuming yields the value the capture point would have returned.\n  hazard multi-resume of a once-continuation duplicates observable effects.\n  favor clarity.',
  '["multi-resume duplicating observable effects","captured state that has since mutated"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'type-class-polymorphism',
  'a uniform interface over several data kinds satisfied by a declared set of required operations; enables abstraction over unrelated types',
  '["concept","function","data"]',
  '["intended","uses","composes","requires","ensures","favor"]',
  '["@Function","@Data"]',
  'the concept <interface> is\n  intended to describe a uniform surface over any data kind that supplies the declared operations.\n  uses the @Function matching "required operation" with at-least 0.9 confidence.\n  composes <operation-a> then <operation-b>.\n  requires every implementing data kind supplies the declared operations.\n  ensures consumers depending on the interface compile against any conforming implementation.\n  favor correctness.',
  '["silent conformance gap when a required operation is merely absent"]',
  '["correctness","clarity"]',
  '[]'
),
(
  'stream-processing-window',
  'a bounded aggregation over a stream of events grouped into tumbling, sliding, or session windows',
  '["function","event","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Event","@Function","@Data"]',
  'the function <aggregate> is\n  intended to produce a per-window aggregate over a stream of timestamped events.\n  uses the @Event matching "incoming event" with at-least 0.9 confidence.\n  uses the @Function matching "reduce" with at-least 0.9 confidence.\n  requires every event carries a timestamp suitable for window assignment.\n  ensures late-arriving events outside the watermark are excluded.\n  hazard missing watermarks cause unbounded in-memory state retention.\n  favor availability.',
  '["missing watermarks","unbounded in-memory windows"]',
  '["availability","auditability"]',
  '[]'
),
(
  'infrastructure-declaration',
  'a declarative specification of infrastructure resources — compute, network, storage — with authored-time validation and apply-time convergence',
  '["concept","data"]',
  '["intended","uses","composes","requires","ensures","favor"]',
  '["@Data"]',
  'the concept <deployment> is\n  intended to describe a target infrastructure state as a set of named resources with declared relationships.\n  uses the @Data matching "resource spec" with at-least 0.9 confidence.\n  composes <network> then <compute> then <storage>.\n  requires every resource references only declared peers.\n  ensures a diff against live state yields a minimal convergent plan.\n  favor reproducibility.',
  '["hidden resource dependencies discovered only at apply time"]',
  '["reproducibility","auditability"]',
  '[]'
),
(
  'structured-imperative-block',
  'a sequence of imperative statements with lexical scope, declared local variables, and explicit control flow — the substrate for low-level systems authoring',
  '["function"]',
  '["intended","requires","ensures","favor"]',
  '[]',
  'the function <step> is\n  intended to execute a bounded sequence of imperative statements with declared local scope.\n  requires every local variable is initialized before use.\n  ensures every declared loop has a visible termination measure.\n  favor totality.',
  '["uninitialized locals read before write","unbounded loops without declared variant"]',
  '["totality","clarity"]',
  '[]'
),
(
  'logic-programming-rule',
  'a head-and-body rule that derives new facts from existing ones under declared constraints; the substrate for rule engines and query engines',
  '["property","data"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Data"]',
  'the property <rule> is\n  intended to assert that whenever the body conditions hold, the head fact is derivable.\n  generator tuples from the declared fact store.\n  uses the @Data matching "base fact" with at-least 0.9 confidence.\n  requires the rule terminates on any finite fact store.\n  ensures the derived fact set is closed under repeated application.\n  favor determinism.',
  '["non-terminating recursion under unrestricted negation"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'term-rewriting-semantics',
  'operational semantics expressed as a set of term-rewriting rules over a concrete syntax; the substrate for formal language specification',
  '["property","function","data"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Data","@Function"]',
  'the property <rewrite> is\n  intended to assert that the declared rewrite rule reduces a matching term to its normal form.\n  generator well-typed terms drawn from the declared concrete syntax.\n  uses the @Data matching "term shape" with at-least 0.9 confidence.\n  requires the rewrite system is confluent over the term domain.\n  ensures every accepted term reduces to a unique normal form in finitely many steps.\n  favor correctness.',
  '["non-confluent rewrite rules producing divergent normal forms"]',
  '["correctness","determinism"]',
  '[]'
),
(
  'macro-expansion',
  'a source-level shape that expands into a larger source pattern at authoring time, enabling syntactic abstraction without runtime cost',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <expand> is\n  intended to rewrite a compact authoring shape into a larger canonical source pattern at compile time.\n  uses the @Data matching "input shape" with at-least 0.9 confidence.\n  requires the expansion is hygiene-preserving over the enclosing scope.\n  ensures the expanded source parses cleanly under the same grammar.\n  hazard name capture in the expansion shadows names in the surrounding scope.\n  favor clarity.',
  '["name capture","expansion that no longer parses"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'dependent-type-indexed',
  'a data shape whose type carries a value — a length, a tag, a proof — that the type system checks at authoring time',
  '["data","property"]',
  '["intended","exposes","generator","ensures","favor"]',
  '[]',
  'the data <Vec> is\n  intended to describe a collection whose element type and length are both part of its shape.\n  exposes element_type as identifier.\n  exposes length as natural.\n  exposes elements as list of reference to element_type.\n  favor totality.',
  '["length mismatch silently zero-fills","value-in-type that diverges from a runtime value"]',
  '["totality","correctness"]',
  '[]'
);
-- ── Schema version stamp ────────────────────────────────────────────

INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('baseline_version', '1.0');

COMMIT;
