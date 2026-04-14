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
),
(
  'dimensional-analysis',
  'a quantity whose type carries its physical units so that arithmetic only combines compatible units and yields a well-typed result quantity',
  '["data","function"]',
  '["intended","exposes","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the data <Quantity> is\n  intended to represent a physical measurement carrying both a numeric magnitude and a unit.\n  exposes magnitude as real.\n  exposes unit as identifier.\n  favor numerical_stability.',
  '["silent unit coercion across incompatible dimensions"]',
  '["numerical_stability","correctness"]',
  '[]'
),
(
  'singleton-per-app',
  'a resource the app must declare exactly once — a database, an auth provider, a metrics sink — enforced by the authoring-time cardinality check',
  '["data","concept"]',
  '["intended","exposes","requires","ensures","favor"]',
  '["@Data"]',
  'the data <AppDb> is\n  intended to identify the single authoritative database the app reads and writes.\n  exposes connection_spec as text.\n  favor availability.',
  '["two sibling declarations shadowing each other at merge time"]',
  '["availability","auditability"]',
  '[]'
),
(
  'idempotent-command',
  'a write operation safe to retry because repeated application yields the same observable state as a single application',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <apply> is\n  intended to produce a target state transition that is safe to retry on transient failure.\n  uses the @Data matching "command" with at-least 0.9 confidence.\n  requires the command carries a stable correlation identifier.\n  ensures repeated application with the same identifier yields a single state transition.\n  hazard correlation-identifier reuse across distinct logical commands silently conflates them.\n  favor correctness.',
  '["correlation-identifier reuse","non-idempotent side-effects hidden inside a retry-safe wrapper"]',
  '["correctness","availability"]',
  '[]'
),
(
  'authorization-guard',
  'a capability-based access check gating an operation on a principal holding the declared permission for the declared resource',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <guard> is\n  intended to refuse every operation whose caller lacks the declared capability for the target resource.\n  uses the @Data matching "capability" with at-least 0.9 confidence.\n  requires the capability was issued by the authorized issuer.\n  ensures every denial emits an auditable record with the caller, resource, and denied permission.\n  hazard ambient capabilities escape the declared scope through reference leakage.\n  favor auditability.',
  '["ambient capability leakage","silent permission downgrade on rejection"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'lifecycle-managed-resource',
  'a resource whose acquire and release are lexically paired so the release always runs even on failure paths',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <use> is\n  intended to perform a bounded action against a resource whose release runs on every exit path.\n  uses the @Data matching "resource handle" with at-least 0.9 confidence.\n  requires the acquire call has a lexically-matched release call.\n  ensures the release runs exactly once on every exit path including failure.\n  hazard a release that escapes its lexical scope may double-release or skip the release.\n  favor correctness.',
  '["double release","skipped release on rare failure paths"]',
  '["correctness","availability"]',
  '[]'
),
(
  'event-sourced-state',
  'a state machine whose current value is derived by replaying an append-only log of events; the log is the source of truth and projections are pure functions over it',
  '["concept","event","data","function"]',
  '["intended","uses","composes","ensures","hazard","favor"]',
  '["@Event","@Data","@Function"]',
  'the concept <ledger> is\n  intended to derive current state by folding an append-only log of events through a pure projection.\n  uses the @Event matching "recorded event" with at-least 0.9 confidence.\n  uses the @Function matching "apply event" with at-least 0.9 confidence.\n  composes <append> then <project>.\n  ensures the projection is deterministic over the declared log prefix.\n  hazard a projection that consults external state drifts silently from the log-derived truth.\n  favor auditability.',
  '["projections reading outside state","lossy event compaction"]',
  '["auditability","reproducibility"]',
  '[]'
),
(
  'circuit-breaker',
  'a fault-isolation guard that short-circuits repeated calls to a failing downstream after a threshold is crossed, letting the downstream recover before traffic resumes',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <call> is\n  intended to protect callers from repeated failures of a downstream dependency.\n  uses the @Data matching "breaker state" with at-least 0.9 confidence.\n  uses the @Function matching "downstream" with at-least 0.9 confidence.\n  requires the breaker reads a monotonic clock for timeout decisions.\n  ensures while the breaker is open, downstream calls are refused without invocation.\n  hazard a breaker that never resets permanently strands a healed downstream.\n  favor availability.',
  '["stuck-open breaker","flapping between open and closed under load"]',
  '["availability","auditability"]',
  '[]'
),
(
  'cache-memoization',
  'a pure-function result cache keyed on input equality; repeated calls with the same input return the stored result without re-executing the body',
  '["function","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <memoize> is\n  intended to return the cached result for a previously-seen input and compute-and-cache for a new input.\n  uses the @Data matching "cache store" with at-least 0.9 confidence.\n  uses the @Function matching "pure body" with at-least 0.9 confidence.\n  requires the wrapped function is referentially transparent.\n  ensures for every input, the cached and freshly-computed results are equal.\n  hazard wrapping a non-pure function silently returns stale results.\n  favor performance.',
  '["wrapping non-pure functions","unbounded cache growth without eviction"]',
  '["performance","determinism"]',
  '[]'
),
(
  'publish-subscribe-fanout',
  'a topic-based event distribution where one publisher delivers to zero or more subscribers matching a declared filter, decoupling producers from consumers',
  '["concept","event","function"]',
  '["intended","uses","composes","ensures","hazard","favor"]',
  '["@Event","@Function"]',
  'the concept <topic> is\n  intended to deliver every published event to every subscriber whose filter matches.\n  uses the @Event matching "published" with at-least 0.9 confidence.\n  uses the @Function matching "subscriber filter" with at-least 0.9 confidence.\n  composes <publish> then <match> then <deliver>.\n  ensures delivery to each matching subscriber is at-least-once under normal operation.\n  hazard a slow subscriber back-pressures the whole topic without isolation.\n  favor availability.',
  '["slow-subscriber back-pressure","duplicate delivery on retry"]',
  '["availability","auditability"]',
  '[]'
),
(
  'scheduled-cron-task',
  'a work unit fired on a declared time schedule — interval, cron expression, or clock event — with explicit semantics for missed and overlapping firings',
  '["function","event","data"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the function <tick> is\n  intended to perform a bounded work unit on every scheduled firing.\n  uses the @Data matching "schedule spec" with at-least 0.9 confidence.\n  uses the @Function matching "work body" with at-least 0.9 confidence.\n  requires the schedule spec declares how missed firings are handled.\n  ensures two overlapping firings do not execute concurrently.\n  hazard a long work body silently drops subsequent firings under the missed-firing policy.\n  favor reproducibility.',
  '["missed firings silently dropped","clock skew across executors"]',
  '["reproducibility","auditability"]',
  '[]'
);

-- Parallel-seeded batches — concurrency, distributed-systems, UI/UX
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'task-parallel-fork-join',
  'split work into independent subtasks then rejoin for a combined result',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function compute_sum_in_parallel is\n  intended to fold a large sequence by splitting it into disjoint chunks, folding each chunk independently, and combining partial results.\n  uses the @Function matching "spawn a child task on the worker pool" with at-least 0.9 confidence.\n  uses the @Function matching "wait for all spawned children to complete" with at-least 0.9 confidence.\n  requires the fold operation is associative.\n  ensures the combined result equals the sequential fold.\n  hazard uneven chunk sizes starve some workers.\n  favor performance.',
  '["children panicking must not orphan siblings","chunk size below threshold adds overhead"]',
  '["performance","correctness","determinism"]',
  '[]'
),
(
  'async-awaited-task',
  'describe a long-running task whose result is awaited without blocking a worker',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the function fetch_then_transform is\n  intended to request a remote payload, await its arrival, and return the transformed body without holding a worker thread while waiting.\n  uses the @Function matching "suspend until an external result is ready" with at-least 0.9 confidence.\n  requires the await point is inside a task-capable scope.\n  ensures the returned value is produced only after the remote payload has arrived.\n  hazard awaiting inside a held lock serializes every task.\n  favor responsiveness.',
  '["forgetting to propagate cancellation","creating hidden synchronous callers via blocking await"]',
  '["responsiveness","latency","correctness"]',
  '[]'
),
(
  'actor-mailbox-message',
  'isolate state inside an actor that serially processes one message at a time',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Event","@Data"]',
  'the concept order_clerk is\n  intended to own an order ledger and mutate it only by consuming messages drawn one at a time from a private mailbox.\n  uses the @Event matching "message arrives in the mailbox" with at-least 0.9 confidence.\n  composes the @Data matching "bounded first-in-first-out mailbox" with at-least 0.9 confidence.\n  requires no outside caller holds a reference to the ledger.\n  ensures at most one message for order_clerk is being processed at any instant.\n  hazard unbounded mailboxes grow without backpressure.\n  favor correctness.',
  '["leaking mutable state out through reply payloads","reentrant self-sends causing starvation"]',
  '["correctness","determinism","availability"]',
  '[]'
),
(
  'transactional-memory-block',
  'group multiple shared-memory edits into an atomic commit that retries on conflict',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function transfer_credit is\n  intended to move a balance between two accounts so that either both updates are observed or neither is.\n  uses the @Function matching "begin an optimistic transactional scope" with at-least 0.9 confidence.\n  uses the @Function matching "commit the transactional scope or retry on conflict" with at-least 0.9 confidence.\n  requires every touched cell is a transactional reference.\n  ensures no intermediate state is observable to another transaction.\n  hazard performing non-transactional side effects inside the block duplicates them on retry.\n  favor correctness.',
  '["long read sets starve under contention","side effects cannot be rolled back"]',
  '["correctness","determinism","clarity"]',
  '[]'
),
(
  'lock-free-atomic-counter',
  'maintain a shared counter using atomic arithmetic instead of mutual exclusion',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Function"]',
  'the data request_counter is\n  intended to hold a non-negative integer that every worker may increment concurrently without taking a lock.\n  exposes increment as function.\n  exposes read_current as function.\n  favor performance.',
  '["reordering around the counter breaks causality of surrounding reads","wraparound on unsigned overflow"]',
  '["performance","correctness","latency"]',
  '[]'
),
(
  'barrier-synchronization-point',
  'hold every participant at a shared point until all have arrived, then release them together',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the function release_at_barrier is\n  intended to suspend each participating worker at a rendezvous point and resume them only once every expected participant has arrived.\n  uses the @Function matching "register arrival at a shared rendezvous" with at-least 0.9 confidence.\n  requires the participant count is fixed before the first arrival.\n  ensures no participant observes post-barrier state before every other has reached the barrier.\n  hazard a participant that crashes before arriving deadlocks the rest.\n  favor correctness.',
  '["reusing the barrier without resetting between phases","miscounting participants after dynamic join"]',
  '["correctness","determinism","clarity"]',
  '[]'
),
(
  'read-write-lock-partition',
  'allow many concurrent readers or one exclusive writer of a shared value',
  '["concept"]',
  '["intended","uses","exposes","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the concept guarded_catalog is\n  intended to protect a shared catalog so that either many observers read it in parallel or exactly one mutator edits it alone.\n  uses the @Data matching "shared-exclusive lock primitive" with at-least 0.9 confidence.\n  exposes acquire_shared as function.\n  exposes acquire_exclusive as function.\n  requires no thread upgrades a shared hold into an exclusive hold without releasing first.\n  ensures no reader observes a partially applied mutation.\n  hazard writer starvation when readers continuously arrive.\n  favor correctness.',
  '["nested acquisition causing self-deadlock","holding the exclusive mode across long input/output"]',
  '["correctness","performance","availability"]',
  '[]'
),
(
  'compare-and-swap-update',
  'update a shared cell by re-reading and retrying until a compare succeeds',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function update_head_pointer is\n  intended to replace a shared reference with a derived new value, retrying whenever another participant has changed it in the meantime.\n  uses the @Function matching "atomic compare-and-swap on a shared cell" with at-least 0.9 confidence.\n  requires the derivation function is pure and side-effect free.\n  ensures the final stored value is some derivation of a value that was observed live at the moment of commit.\n  hazard the derivation is silently rerun on each retry, so observable side effects inside it multiply.\n  favor correctness.',
  '["stale-recycled-value confusion where an identical bit pattern hides a reuse","unbounded retry under heavy contention"]',
  '["correctness","performance","determinism"]',
  '[]'
),
(
  'dataflow-future-promise',
  'publish a placeholder now whose value is supplied later by a separate producer',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Function"]',
  'the data pending_result is\n  intended to stand in for a value that is not yet computed but will be fulfilled exactly once by a designated producer.\n  exposes fulfill as function.\n  exposes await_fulfillment as function.\n  favor clarity.',
  '["fulfilling twice violates single-assignment","awaiting without any producer path causes permanent block"]',
  '["clarity","correctness","responsiveness"]',
  '[]'
),
(
  'bounded-channel-handoff',
  'hand off items between producers and consumers through a fixed-capacity queue that applies backpressure',
  '["concept"]',
  '["intended","uses","composes","exposes","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept work_handoff is\n  intended to move items from producers to consumers through a queue of fixed capacity, blocking producers when full and consumers when empty.\n  uses the @Data matching "fixed-capacity first-in-first-out queue" with at-least 0.9 confidence.\n  composes the @Function matching "send an item, waiting if the queue is full" with at-least 0.9 confidence.\n  exposes receive as function.\n  ensures items are delivered in the order they were sent from a single producer.\n  hazard closing the queue while senders are parked loses in-flight sends unless drained.\n  favor availability.',
  '["sizing capacity far below producer burst defeats throughput","multi-producer ordering is only per-producer, not global"]',
  '["availability","responsiveness","latency"]',
  '[]'
),
(
  'leader-election',
  'single coordinator chosen among peers via term-bounded vote',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Event"]',
  'the concept cluster_leader is\n  intended to choose exactly one coordinator per term across a peer set.\n  uses the @Function matching "advance-term" with at-least 0.9 confidence.\n  composes propose_candidacy then collect_votes then commit_leader.\n  requires a monotonically increasing term number per peer.\n  ensures at most one leader is committed per term.\n  hazard split-vote under symmetric timeouts can stall progress.\n  favor correctness.',
  '["split-vote stalls progress","clock-skew shortens lease"]',
  '["correctness","availability","determinism"]',
  '[]'
),
(
  'quorum-write',
  'write is durable once acknowledged by a majority of replicas',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the concept majority_write is\n  intended to acknowledge a write only after a strict majority of replicas persist it.\n  uses the @Function matching "append-to-replica" with at-least 0.9 confidence.\n  composes fan_out_proposal then await_majority_ack then commit_locally.\n  requires replica count to be known and odd-preferred.\n  ensures any two committed writes intersect on at least one replica.\n  hazard minority-partition writes silently lose acknowledgement.\n  favor correctness.',
  '["minority-partition loses writes","slow-replica tail-latency"]',
  '["correctness","availability","auditability"]',
  '[]'
),
(
  'gossip-protocol',
  'membership and state disseminate via periodic random peer exchange',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Event"]',
  'the concept peer_gossip is\n  intended to propagate membership and versioned state by periodic random peer exchange.\n  uses the @Function matching "pick-random-peer" with at-least 0.9 confidence.\n  composes select_peer then exchange_digest then merge_versioned_state.\n  requires each state entry to carry a version or timestamp.\n  ensures with probability one every reachable peer converges on the latest state.\n  hazard infection fan-out too low leaves stale peers indefinitely.\n  favor availability.',
  '["stale-peers under low fanout","bandwidth-amplification on large state"]',
  '["availability","responsiveness","reproducibility"]',
  '[]'
),
(
  'consistent-hash-partition',
  'keys map to nodes on a ring so that node changes remap only a fraction of keys',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the concept ring_partition is\n  intended to assign keys to nodes on a ring so that adding or removing a node remaps only a bounded fraction of keys.\n  uses the @Function matching "hash-key-to-ring" with at-least 0.9 confidence.\n  composes hash_key then locate_successor_node then replicate_to_virtual_nodes.\n  requires a uniform hash and a ring populated with virtual node tokens.\n  ensures the expected fraction of remapped keys on a single node change is one over node count.\n  hazard a hot key concentrates load on one successor regardless of ring balance.\n  favor performance.',
  '["hot-key concentrates load","uneven virtual-node distribution"]',
  '["performance","availability","determinism"]',
  '[]'
),
(
  'two-phase-commit',
  'coordinator drives prepare then commit across participants with atomic outcome',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Event"]',
  'the concept atomic_commit is\n  intended to commit a transaction across participants atomically via a prepare phase then a decision phase.\n  uses the @Function matching "record-prepare-vote" with at-least 0.9 confidence.\n  composes broadcast_prepare then collect_votes then broadcast_decision.\n  requires durable logging of the prepared state on every participant before voting yes.\n  ensures all participants agree on commit-or-abort for a given transaction id.\n  hazard coordinator failure after prepare leaves participants blocked until recovery.\n  favor correctness.',
  '["coordinator-failure blocks participants","prepared-state holds locks"]',
  '["correctness","auditability","totality"]',
  '[]'
),
(
  'saga-compensating-transaction',
  'long-running workflow committed as steps each with a defined compensation',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function"]',
  'the concept saga_workflow is\n  intended to execute a long workflow as a sequence of local commits each paired with a compensating action on later failure.\n  uses the @Function matching "run-forward-step" with at-least 0.9 confidence.\n  composes run_forward_step then on_failure run_compensation_in_reverse.\n  requires each forward step to have an effect-reversing compensation that is safe to retry.\n  ensures on any step failure the visible effects of earlier steps are undone by compensations.\n  hazard compensation is not a true inverse when external observers already acted on intermediate state.\n  favor correctness.',
  '["compensation not a true inverse","observer reads intermediate state"]',
  '["correctness","auditability","totality"]',
  '[]'
),
(
  'vector-clock-causality',
  'each event carries a per-peer counter vector so causal order can be decided',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the concept causal_clock is\n  intended to attach a per-peer counter vector to every event so causal precedence and concurrency can be decided pairwise.\n  uses the @Function matching "increment-local-component" with at-least 0.9 confidence.\n  composes increment_local_component then stamp_outgoing_event then merge_on_receive.\n  requires a stable peer identity set for the vector components.\n  ensures two events are concurrent exactly when neither vector dominates the other.\n  hazard vector size grows linearly with the number of participating peers.\n  favor correctness.',
  '["vector size grows with peers","peer-identity churn breaks comparison"]',
  '["correctness","auditability","determinism"]',
  '[]'
),
(
  'read-repair',
  'reader reconciles replica divergence inline by writing back the freshest value',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Event"]',
  'the concept inline_repair is\n  intended to reconcile divergent replicas during a read by writing the freshest value back to stale replicas inline.\n  uses the @Function matching "compare-replica-versions" with at-least 0.9 confidence.\n  composes query_replicas then pick_freshest_by_version then write_back_to_stale.\n  requires values to carry a totally orderable version token.\n  ensures after a successful repairing read every contacted replica holds the freshest version.\n  hazard repair writes amplify load on the read path under skew.\n  favor correctness.',
  '["repair amplifies read-load","partial repair under partition"]',
  '["correctness","availability","latency"]',
  '[]'
),
(
  'conflict-free-replicated-data',
  'replicas merge concurrent updates commutatively without a coordinator',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the concept mergeable_replica is\n  intended to let replicas apply concurrent updates locally and merge pairwise without coordination.\n  uses the @Function matching "merge-replica-state" with at-least 0.9 confidence.\n  composes apply_local_update then gossip_state_delta then merge_pairwise.\n  requires the merge operation to be commutative associative and idempotent.\n  ensures any two replicas that have seen the same set of updates converge to the same state.\n  hazard tombstone accumulation inflates state size over time.\n  favor correctness.',
  '["tombstone state growth","merge requires monotone lattice"]',
  '["correctness","availability","determinism"]',
  '[]'
),
(
  'eventual-consistency-claim',
  'quiescent replicas converge to the same state given finite delivery',
  '["property"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Function"]',
  'the property convergence_claim is\n  intended to assert that if updates stop and every message is eventually delivered then all replicas converge to the same state.\n  generator a random interleaving of updates across replicas followed by a quiescent delivery tail.\n  uses the @Function matching "step-replica-exchange" with at-least 0.9 confidence.\n  requires a finite update stream and a fair delivery schedule.\n  ensures under quiescence every replica reports an identical observable state.\n  favor correctness.',
  '["unbounded delay never reaches quiescence","observable-state definition drift"]',
  '["correctness","reproducibility","determinism"]',
  '[]'
),
(
  'responsive-layout-grid',
  'arrange content regions that reflow across viewport sizes without hiding functionality',
  '["screen"]',
  '["intended","uses","exposes","favor"]',
  '["@Data"]',
  'the screen <name> is\n  intended to present primary content and secondary controls across narrow, medium, and wide viewports without functional loss.\n  uses the @Data matching "viewport dimensions and breakpoint thresholds" with at-least 0.9 confidence.\n  exposes region as named layout slot.\n  exposes breakpoint as viewport threshold.\n  favor accessibility.',
  '["hidden overflow traps content at narrow widths","fixed pixel breakpoints ignore user font-size preferences","reordering for small screens hides primary actions below the fold"]',
  '["accessibility","responsiveness","portability"]',
  '[]'
),
(
  'focus-trap-dialog',
  'hold keyboard focus inside a modal surface until it is dismissed and restore prior focus on close',
  '["screen"]',
  '["intended","uses","exposes","favor"]',
  '["@Event","@Data"]',
  'the screen <name> is\n  intended to confine keyboard focus within a modal region while it is open and return focus to the invoking control on close.\n  uses the @Data matching "ordered list of focusable descendants and the element that opened the dialog" with at-least 0.95 confidence.\n  uses the @Event matching "forward and backward focus advance requests while the dialog is open" with at-least 0.95 confidence.\n  exposes open_request as dialog invocation.\n  exposes dismiss_request as dialog close.\n  favor accessibility.',
  '["focus escapes to background content via tab cycling","invoker focus is not restored on close","nested dialogs collide over the trap"]',
  '["accessibility","correctness","determinism"]',
  '[]'
),
(
  'optimistic-ui-update',
  'apply a user-initiated change to the visible state immediately and reconcile with the authoritative result when it arrives',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Event","@Function"]',
  'the concept <name> is\n  intended to reflect a user-initiated change in the visible state before confirmation and reconcile when the authoritative result arrives.\n  uses the @Data matching "pending mutation record with local predicted state and original state" with at-least 0.95 confidence.\n  uses the @Event matching "authoritative acknowledgment or rejection of a pending mutation" with at-least 0.95 confidence.\n  composes the @Function matching "rollback that restores the original state when a pending mutation is rejected" with at-least 0.95 confidence.\n  requires every pending mutation to carry both the predicted state and the original state.\n  ensures rejection restores the state that existed before the pending mutation was applied.\n  hazard stacked pending mutations rollback in a wrong order and corrupt the visible state.\n  favor responsiveness.',
  '["rollback order mismatch when multiple mutations are pending","reconciliation overwrites newer local edits","the user perceives success before durable persistence"]',
  '["responsiveness","correctness","clarity"]',
  '[]'
),
(
  'form-validation-client-side',
  'evaluate entered field values against declared constraints and surface failures at the field before submission',
  '["concept"]',
  '["intended","uses","composes","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to evaluate entered field values against declared constraints and surface each failure at the owning field before submission is permitted.\n  uses the @Data matching "field descriptor with identifier, declared constraints, current value, and current error set" with at-least 0.95 confidence.\n  composes the @Function matching "pure evaluator that maps a value and a constraint set to an error set" with at-least 0.95 confidence.\n  ensures submission is blocked while any field carries a non-empty error set.\n  ensures the error set for each field is visible at or adjacent to the field.\n  hazard client evaluation is treated as sufficient and the server skips the same checks.\n  favor correctness.',
  '["client evaluation relied on as the only gate","error messages blame the user without guidance","validation triggers on every keystroke and overwhelms assistive technology"]',
  '["correctness","accessibility","clarity"]',
  '[]'
),
(
  'progressive-disclosure',
  'reveal secondary controls and detail only when the user signals intent to engage with them',
  '["screen"]',
  '["intended","uses","exposes","favor"]',
  '["@Data","@Event"]',
  'the screen <name> is\n  intended to present primary actions immediately and reveal secondary controls and detail only when the user signals intent to engage with them.\n  uses the @Data matching "disclosure section with primary summary, expanded detail, and current expansion state" with at-least 0.9 confidence.\n  uses the @Event matching "user request to expand or collapse a disclosure section" with at-least 0.9 confidence.\n  exposes summary as always-visible primary view.\n  exposes detail as on-request expanded view.\n  favor clarity.',
  '["hidden detail contains information required to complete the primary task","expansion state is lost on navigation and frustrates return visits","search cannot find text inside collapsed sections"]',
  '["clarity","accessibility","discoverability"]',
  '[]'
),
(
  'keyboard-navigation-map',
  'declare a deterministic traversal order and action bindings for every interactive element reachable without a pointer',
  '["concept"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the concept <name> is\n  intended to declare a deterministic traversal order and action bindings for every interactive element reachable without a pointer.\n  uses the @Data matching "ordered list of interactive elements, each with a declared traversal position and action binding" with at-least 0.95 confidence.\n  requires every interactive element to carry a visible focus indicator while focused.\n  ensures the traversal order matches the reading order presented to the user.\n  hazard a pointer-only control exists that the traversal order cannot reach.\n  favor accessibility.',
  '["pointer-only controls with no keyboard path","focus indicator suppressed by style rules","traversal order diverges from visible reading order"]',
  '["accessibility","determinism","discoverability"]',
  '[]'
),
(
  'undo-redo-history',
  'record user-visible state transitions as a bounded linear history that can be traversed backward and forward',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to record user-visible state transitions as a bounded linear history that the user can traverse backward and forward.\n  uses the @Data matching "ordered history of state transitions with a current position marker" with at-least 0.95 confidence.\n  composes the @Function matching "pure inverse that reverses one recorded transition" with at-least 0.95 confidence.\n  composes the @Function matching "pure replay that re-applies one recorded transition" with at-least 0.95 confidence.\n  requires every recorded transition to declare a total inverse and a total replay.\n  ensures a new user transition discards any transitions that were ahead of the current position.\n  hazard a transition whose inverse depends on external state that has since changed cannot be truly reversed.\n  favor correctness.',
  '["transitions with non-total inverses","external side effects not captured in the inverse","unbounded history exhausts memory"]',
  '["correctness","determinism","clarity"]',
  '[]'
),
(
  'drag-and-drop-surface',
  'let a user pick up a source item and release it onto a compatible target while showing valid drop zones throughout',
  '["screen"]',
  '["intended","uses","exposes","favor"]',
  '["@Event","@Data"]',
  'the screen <name> is\n  intended to let a user pick up a source item and release it onto a compatible target while valid drop zones are indicated throughout the gesture.\n  uses the @Data matching "source item descriptor with item identity and allowed target kinds" with at-least 0.95 confidence.\n  uses the @Event matching "pickup, hover-over-target, and release gesture events from either pointer or keyboard" with at-least 0.95 confidence.\n  exposes pickup as gesture start.\n  exposes release as gesture end.\n  favor accessibility.',
  '["pointer-only gesture with no keyboard equivalent","release outside any target leaves the source in an ambiguous state","valid drop zones are not indicated before release"]',
  '["accessibility","correctness","clarity"]',
  '[]'
),
(
  'infinite-scroll-pagination',
  'extend a visible list with additional entries as the user approaches the end while preserving position on return',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Event","@Function"]',
  'the concept <name> is\n  intended to extend a visible list with additional entries as the user approaches the end and to preserve scroll position when the user navigates away and returns.\n  uses the @Data matching "cursor or page token that identifies the next contiguous range of entries" with at-least 0.95 confidence.\n  uses the @Event matching "the visible window approaches the current tail of the loaded range" with at-least 0.9 confidence.\n  composes the @Function matching "fetch that maps a cursor to the next contiguous range and the following cursor" with at-least 0.95 confidence.\n  requires the cursor to be stable under concurrent insertions ahead of the current tail.\n  ensures the user can reach every entry in the underlying collection without a pointer-only gesture.\n  hazard footer content becomes unreachable because new entries load ahead of it forever.\n  favor accessibility.',
  '["footer content unreachable behind endless loads","duplicated entries on concurrent insertions","scroll position lost on return navigation"]',
  '["accessibility","responsiveness","correctness"]',
  '[]'
),
(
  'toast-notification-queue',
  'display transient status messages in a bounded ordered queue without stealing focus from the current task',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the concept <name> is\n  intended to display transient status messages in a bounded ordered queue without stealing focus from the current task.\n  uses the @Data matching "queued message with severity, text, and declared display duration" with at-least 0.95 confidence.\n  uses the @Event matching "enqueue of a new message and expiration of a displayed message" with at-least 0.95 confidence.\n  requires the queue to have a declared maximum concurrent display count.\n  ensures each message is announced to assistive technology without moving keyboard focus.\n  hazard error-severity messages auto-dismiss before the user can read them.\n  favor accessibility.',
  '["critical messages auto-dismiss too early","silent drops on overflow hide important status","focus moves to the toast and disrupts the current task"]',
  '["accessibility","clarity","responsiveness"]',
  '[]'
);

-- ── Schema version stamp ────────────────────────────────────────────

INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('baseline_version', '1.0');

COMMIT;
