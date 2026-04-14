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
-- Corpus-driven extensions: qualities referenced by the archived doc 14
-- translation corpus but absent from the founding 10. Adding them here
-- is what lets the closure proof's parse rate climb above zero.
('correctness',           'semantics',          NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('determinism',           'semantics',          NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('clarity',               'authoring',          NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('documentation',         'authoring',          NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('discoverability',       'authoring',          NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('reproducibility',       'ops',                NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('portability',           'ops',                NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('responsiveness',        'performance',        NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('latency',               'performance',        NULL, 'any',                  NULL,  'doc 14 corpus', NULL),
('performance',           'performance',        NULL, 'any',                  NULL,  'doc 14 corpus', NULL);

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
('proof',        'property', 'doc 14 block 24', 'baseline-1.0', 'universally-quantified theorem claim maps to property kind'),
('composition',  'module',   'doc 14 block 32', 'baseline-1.0', 'composition-of-functions idiom maps to module kind'),
('row',          'data',     'doc 14 block 37', 'baseline-1.0', 'data-table row idiom maps to data kind'),
('diagram',      'screen',   'doc 14 block 47', 'baseline-1.0', 'architecture diagram — screen is the generalized rendered-artifact kind'),
('participants', 'data',     'doc 14 block 47', 'baseline-1.0', 'workflow participant list maps to data kind'),
('layout',       'screen',   'doc 14 block 53', 'baseline-1.0', 'UI layout arrangement maps to screen kind'),
('format',       'data',     'doc 14 block 63', 'baseline-1.0', 'data format specification maps to data kind');

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

-- ── Patterns (canonical authoring shapes extracted from doc 14 corpus) ──
--
-- Each row captures a shape that ≥N v2 translations conform to. AI
-- clients query this table to find the canonical rendering for a
-- given intent. Seed is minimal — three high-frequency shapes.
-- Additional patterns are user-added via `nom grammar add-pattern`
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
  '["doc 14 — majority shape across function translations"]'
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
  '["doc 14 — majority shape across data translations"]'
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
  '["doc 14 — standard concept composition pattern"]'
);

-- ── Schema version stamp ────────────────────────────────────────────

INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('baseline_version', '1.0');

COMMIT;
