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

-- ── Extended kinds — B4 (29 UX/app/media/flow/bench kinds) ─────────
-- `screen` already registered above; remaining 28 are new.

INSERT OR IGNORE INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) VALUES
('ux_pattern',        'A reusable UI/UX interaction pattern describing a recurring problem and its solution shape.', '[]', '[]', 'baseline-1.0-b4', NULL),
('design_rule',       'A visual design constraint or guideline that governs appearance and layout decisions.', '[]', '[]', 'baseline-1.0-b4', NULL),
('user_flow',         'A sequence of steps a user takes to accomplish a goal within an application.', '[]', '[]', 'baseline-1.0-b4', NULL),
('skill',             'A reusable authoring procedure encoding how to accomplish a class of tasks in Nom.', '[]', '[]', 'baseline-1.0-b4', NULL),
('app_manifest',      'Top-level declaration binding an application to its pages, actions, data sources, and target platforms.', '[]', '[]', 'baseline-1.0-b4', NULL),
('data_source',       'Named external or internal data provider with schema and connection parameters.', '[]', '[]', 'baseline-1.0-b4', NULL),
('query',             'A named, parameterized read operation over a data source that returns a typed result set.', '[]', '[]', 'baseline-1.0-b4', NULL),
('app_action',        'A named side-effecting operation an application can invoke in response to user intent.', '[]', '[]', 'baseline-1.0-b4', NULL),
('app_variable',      'A named mutable state cell scoped to an application or page.', '[]', '[]', 'baseline-1.0-b4', NULL),
('page',              'A top-level navigable surface within an application, composed of screens and actions.', '[]', '[]', 'baseline-1.0-b4', NULL),
('benchmark',         'A named performance measurement definition with workload, platform, and metric declarations.', '[]', '[]', 'baseline-1.0-b4', NULL),
('benchmark_run',     'A recorded execution of a benchmark capturing timing moments and custom counters.', '[]', '[]', 'baseline-1.0-b4', NULL),
('flow_artifact',     'A named artifact produced or consumed by a recorded execution flow.', '[]', '[]', 'baseline-1.0-b4', NULL),
('flow_step',         'A single step in a recorded execution flow with start/end timestamps and input/output hashes.', '[]', '[]', 'baseline-1.0-b4', NULL),
('flow_middleware',   'An interceptor installed in an execution flow that observes or transforms steps.', '[]', '[]', 'baseline-1.0-b4', NULL),
('media_unit',        'A single addressable media item: image, audio clip, video clip, or glyph set.', '[]', '[]', 'baseline-1.0-b4', NULL),
('pixel_grid',        'A raster surface defined by width, height, and a color-depth declaration.', '[]', '[]', 'baseline-1.0-b4', NULL),
('audio_buffer',      'A named buffer of audio samples with sample rate and channel layout.', '[]', '[]', 'baseline-1.0-b4', NULL),
('video_stream',      'A named sequence of frames with frame rate, resolution, and codec binding.', '[]', '[]', 'baseline-1.0-b4', NULL),
('vector_path',       'A resolution-independent path described by control points and fill/stroke rules.', '[]', '[]', 'baseline-1.0-b4', NULL),
('glyph_outline',     'A single typographic glyph defined by its outline curves and advance metrics.', '[]', '[]', 'baseline-1.0-b4', NULL),
('mesh_geometry',     'A three-dimensional mesh defined by vertex positions, normals, and face topology.', '[]', '[]', 'baseline-1.0-b4', NULL),
('color',             'A named color value with color-space declaration and component values.', '[]', '[]', 'baseline-1.0-b4', NULL),
('palette',           'An ordered collection of named color entries forming a design color system.', '[]', '[]', 'baseline-1.0-b4', NULL),
('codec',             'A named encode/decode specification binding a media format to its parameter set.', '[]', '[]', 'baseline-1.0-b4', NULL),
('container',         'A named media container format grouping one or more encoded tracks.', '[]', '[]', 'baseline-1.0-b4', NULL),
('media_metadata',    'Structured descriptive data attached to a media unit covering provenance and tags.', '[]', '[]', 'baseline-1.0-b4', NULL),
('render_pipeline',   'A named sequence of rendering stages producing a final output surface from input primitives.', '[]', '[]', 'baseline-1.0-b4', NULL);

-- ── Self-documenting skill entries — B7 (9 skill-kind rows) ─────────
-- Each row documents a reusable authoring procedure as a `skill` kind.

INSERT OR IGNORE INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) VALUES
('author_nom_app',           'Skill: guide for authoring a Nom app from brainstorm to production-ready artifact.', '[]', '[]', 'baseline-1.0-b7', NULL),
('compose_from_dict',        'Skill: compose any artifact using only entries from the dictionary without external references.', '[]', '[]', 'baseline-1.0-b7', NULL),
('debug_nom_closure',        'Skill: systematically debug Nom closure compilation failures using the error-trace pipeline.', '[]', '[]', 'baseline-1.0-b7', NULL),
('extend_nom_compiler',      'Skill: add a new nomtu kind to the compiler, grammar registry, and test suite end-to-end.', '[]', '[]', 'baseline-1.0-b7', NULL),
('ingest_new_ecosystem',     'Skill: ingest a new language ecosystem into nomdict.db via the corpus ingestion pipeline.', '[]', '[]', 'baseline-1.0-b7', NULL),
('use_ai_loop',              'Skill: run the verify-build-bench-flow authoring loop with AI as the intent-resolution oracle.', '[]', '[]', 'baseline-1.0-b7', NULL),
('compose_brutalist_webpage','Skill: compose a brutalist-aesthetic web page using pixel_grid and vector_path primitives.', '[]', '[]', 'baseline-1.0-b7', NULL),
('compose_generative_art',   'Skill: compose a generative art piece via pixel_grid with procedural color and vector_path layers.', '[]', '[]', 'baseline-1.0-b7', NULL),
('compose_lofi_audio_loop',  'Skill: compose a lofi audio loop via audio_buffer with layered instrument and effect bindings.', '[]', '[]', 'baseline-1.0-b7', NULL);

-- ── Composition-target kinds — AH-DB-KINDS (14 hybrid-compose targets) ──
-- Each row is a top-level composition target kind for the hybrid compose system.

INSERT OR IGNORE INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) VALUES
('video_compose',        'Video composition from text or images.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('picture_compose',      'Still image generation from text.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('audio_compose',        'Audio generation from text or parameters.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('presentation_compose', 'Slide presentation from outline.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('web_app_compose',      'Web application from specification.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('mobile_app_compose',   'Mobile application skeleton from specification.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('native_app_compose',   'Native desktop application from specification.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('document_compose',     'Document (PDF or text) from outline.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('data_extract',         'Structured data extraction from unstructured input.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('data_query',           'Semantic data query over structured datasets.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('workflow_compose',     'Workflow automation from natural language description.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('ad_creative_compose',  'Advertising creative from brief.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('mesh_3d_compose',      'Three-dimensional mesh generation from description.', '[]', '[]', 'baseline-1.0-ah-db', NULL),
('storyboard_compose',   'Visual storyboard from narrative.', '[]', '[]', 'baseline-1.0-ah-db', NULL);

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
('benefit',   'clause_opener', '["function","property","event"]', 'doc 04', 'baseline-1.0', 'positive effect valence'),
('retry',     'clause_opener', '["function"]', 'GAP-12', 'baseline-1.0', 'retry-policy clause: at-most N times with strategy backoff'),
('shaped',    'clause_opener', '["data"]', 'GAP-12', 'baseline-1.0', 'pattern-shape clause: shaped like "<pattern>"'),
('accesses',  'clause_opener', '["function"]', 'GAP-12', 'baseline-1.0', 'nested-record-path access clause: accesses <path>[, <path>]*');

-- Ref-slot vocabulary
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('matching',   'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('with',       'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('at-least',   'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL),
('at-most',    'ref_slot', NULL, 'GAP-12', 'baseline-1.0', 'retry-policy count bound'),
('confidence', 'ref_slot', NULL, 'doc 04', 'baseline-1.0', NULL);

-- Retry-policy strategy words (used inside retry clauses; not clause openers themselves)
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('times',       'clause_arg', '["function"]', 'GAP-12', 'baseline-1.0', 'multiplier word in retry at-most N times'),
('backoff',     'clause_arg', '["function"]', 'GAP-12', 'baseline-1.0', 'trailing word in retry with <strategy> backoff'),
('exponential', 'clause_arg', '["function"]', 'GAP-12', 'baseline-1.0', 'exponential backoff strategy'),
('linear',      'clause_arg', '["function"]', 'GAP-12', 'baseline-1.0', 'linear backoff strategy'),
('fixed',       'clause_arg', '["function"]', 'GAP-12', 'baseline-1.0', 'fixed backoff strategy');

-- Pattern-shape clause args (used inside shaped like clauses)
INSERT OR IGNORE INTO keywords (token, role, kind_scope, source_ref, shipped_commit, notes) VALUES
('like', 'clause_arg', '["data"]', 'GAP-12', 'baseline-1.0', 'preposition in shaped like "<pattern>"');

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
-- function (13 clauses): intended (req) / uses / requires / ensures (≥1 req) / hazard / favor / retry / format / accesses / watermark / window / clock / quality
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('function', 'intended',  1, 1,  '''intended to'' <prose-sentence> ''.''', 'doc 04'),
('function', 'uses',      0, 2,  '''uses the'' ''@'' Kind ''matching'' <quoted-prose> ''with at-least'' <0..1> ''confidence'' ''.''', 'doc 04'),
('function', 'requires',  0, 3,  '''requires'' <prose> ''.''', 'doc 04'),
('function', 'ensures',   2, 4,  '''ensures'' <prose> ''.''', 'doc 04'),
('function', 'hazard',    0, 5,  '''hazard'' <prose> ''.''', 'doc 04'),
('function', 'favor',     0, 6,  '''favor'' <quality-name> ''.''', 'doc 08'),
('function', 'retry',     0, 7,  '''retry at-most'' <N> ''times'' (''with'' (''exponential''|''linear''|''fixed'') ''backoff'')? ''.''', 'GAP-12'),
('function', 'format',    0, 8,  '''format'' <quoted-template-with-{interpolation}> ''.''', 'GAP-12'),
('function', 'accesses',  0, 9,  '''accesses'' <dot-path> (''[,'' <dot-path>]*)? ''.''', 'GAP-12'),
('function', 'watermark', 0, 10, '''watermark'' <field> ''lag'' <N> ''seconds'' ''.''', 'GAP-12'),
('function', 'window',    0, 11, '''window'' (''tumbling''|''sliding''|''session'') <N> ''seconds'' ''.''', 'GAP-12'),
('function', 'clock',     0, 12, '''clock domain'' <quoted-name> ''at'' <N> ''mhz'' ''.''', 'GAP-12'),
('function', 'quality',   0, 13, '''quality'' <quality-name> <0..1> ''.''', 'GAP-12');

-- data (5 clauses): intended (req) / exposes (≥1 req) / favor / shaped / field-tag
INSERT OR IGNORE INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) VALUES
('data', 'intended',  1, 1, '''intended to'' <prose> ''.''', 'doc 04'),
('data', 'exposes',   2, 2, '''exposes'' <field> (''at tag'' <int>)? ''as'' <type> ''.''', 'doc 04'),
('data', 'favor',     0, 3, '''favor'' <quality-name> ''.''', 'doc 08'),
('data', 'shaped',    0, 4, '''shaped like'' <quoted-pattern> ''.''', 'GAP-12'),
('data', 'field_tag', 0, 5, '''field'' <field-name> ''tagged'' <quoted-wire-name> ''.''', 'GAP-12');

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


-- Parallel-seeded batch 2 — security + testing
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'content-signature-verification',
  'verify a blob carries a matching signature from a trusted key before use',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to reject any blob whose signature does not match a trusted public key.\n  uses the @Data matching "trusted key set" with at-least 0.9 confidence.\n  requires the signature to cover the full blob and no separator ambiguity to exist.\n  ensures only blobs with a valid signature by a currently-trusted key return ok.\n  hazard accepting a signature over a prefix while using the suffix leaks forgery.\n  favor auditability.',
  '["signature-over-prefix forgery","trust set drift","algorithm downgrade"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'password-hash-storage',
  'store a user secret as a salted slow-hash digest, never in recoverable form',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to persist a user secret only as a per-record salted slow-hash digest.\n  uses the @Data matching "slow hash parameters" with at-least 0.9 confidence.\n  requires a fresh random salt per record and a tuned work factor.\n  ensures the stored row never contains the original secret nor a fast-hash of it.\n  hazard reusing salts or using a fast hash collapses offline cracking cost.\n  favor auditability.',
  '["salt reuse","fast-hash shortcut","work factor too low"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'rate-limit-per-principal',
  'cap the request arrival rate per authenticated caller over a sliding window',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to reject requests from a principal that exceed a configured rate over a sliding window.\n  uses the @Data matching "per-principal counter window" with at-least 0.9 confidence.\n  requires the principal identity to be verified before the counter is consulted.\n  ensures a principal cannot exceed the configured budget within any window of the configured span.\n  hazard keying the counter by network address instead of principal lets shared clients starve each other.\n  favor availability.',
  '["wrong key dimension","window boundary burst","counter skew across replicas"]',
  '["availability","auditability"]',
  '[]'
),
(
  'input-sanitization-boundary',
  'parse and validate untrusted input at a single boundary before it enters the domain',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to convert untrusted bytes into a typed domain value or a structured rejection.\n  uses the @Data matching "validated domain value" with at-least 0.9 confidence.\n  requires every untrusted source to pass through this boundary exactly once.\n  ensures no downstream consumer receives bytes that have not been parsed and range-checked here.\n  hazard re-entering unsanitized input on a secondary path bypasses the boundary entirely.\n  favor correctness.',
  '["secondary unsanitized path","double-decode confusion","late validation"]',
  '["correctness","auditability"]',
  '[]'
),
(
  'secret-rotation-window',
  'rotate a long-lived secret on a schedule with a bounded dual-acceptance window',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to replace an active secret with a freshly generated one on a fixed schedule.\n  uses the @Data matching "active and previous secret pair" with at-least 0.9 confidence.\n  requires both the new and previous secret to be accepted for a bounded overlap window.\n  ensures after the overlap expires only the new secret is accepted anywhere.\n  hazard leaving the previous secret accepted beyond the overlap defeats the rotation.\n  favor auditability.',
  '["unbounded overlap","rotation skipped on failure","previous secret leaked during overlap"]',
  '["auditability","availability"]',
  '[]'
),
(
  'audit-trail-append-only',
  'record every sensitive action to a log that admits appends but not edits or deletes',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to append an immutable record describing a sensitive action, its actor, and its time.\n  uses the @Data matching "append-only log segment" with at-least 0.9 confidence.\n  requires each record to carry a hash chained to the previous record.\n  ensures a later modification of any record breaks the chain and is detectable.\n  hazard permitting in-place edits or truncation silently erases evidence.\n  favor auditability.',
  '["in-place edit","truncation","chain gap on crash"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'capability-token-scoped',
  'issue a token that names exactly the actions and resources it grants, nothing more',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to mint a token whose payload enumerates the actions and resources it permits.\n  uses the @Data matching "scoped capability token" with at-least 0.9 confidence.\n  requires every downstream check to verify the action and resource are within the token scope.\n  ensures a token cannot authorize any action or resource outside its stated scope.\n  hazard ambient identity checks that ignore the scope collapse the capability into a bearer key.\n  favor auditability.',
  '["ambient identity override","scope widening on refresh","wildcard scope"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'time-bounded-credential',
  'issue a credential that carries its own expiry and is rejected after it',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to mint a credential whose payload states a not-after timestamp.\n  uses the @Data matching "time-bounded credential" with at-least 0.9 confidence.\n  requires every verifier to read a trusted clock and compare against the not-after field.\n  ensures a credential presented after its not-after is rejected everywhere.\n  hazard trusting a clock supplied by the caller lets an expired credential be replayed forever.\n  favor auditability.',
  '["caller-supplied clock","clock skew tolerance too wide","missing not-after"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'encrypted-at-rest-data',
  'store persistent data only under a key managed outside the storage layer',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to write persistent records only after encryption under a key held outside the storage layer.\n  uses the @Data matching "data encryption key handle" with at-least 0.9 confidence.\n  requires the key handle to be resolvable only by authorized callers at read time.\n  ensures a raw read of the storage medium yields ciphertext that is useless without the key.\n  hazard caching the plaintext key inside the storage process defeats the separation.\n  favor auditability.',
  '["key co-location with ciphertext","plaintext cache in storage process","unencrypted backup path"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'cross-origin-request-gating',
  'accept cross-origin calls only from an allow-listed origin set with explicit methods',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to accept a cross-origin call only if its origin and method appear on an allow list.\n  uses the @Data matching "origin allow list" with at-least 0.9 confidence.\n  requires the allow list to enumerate exact origins and exact methods with no wildcards.\n  ensures a call from an origin or method not on the list is refused before any side effect.\n  hazard reflecting the caller-supplied origin into the allow response turns the gate into a pass-through.\n  favor auditability.',
  '["origin reflection","wildcard allow","preflight bypass"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'unit-test-assertion',
  'single deterministic claim that one input shape produces one expected output shape',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Function","@Data"]',
  'the scenario <name> is\n  intended to pin one observable fact about the subject under a fixed input.\n  given the @Data matching "prepared input fixture" with at-least 0.9 confidence.\n  when the @Function matching "subject under test" with at-least 0.9 confidence runs against that input.\n  then the result equals the expected value exactly.\n  favor correctness.',
  '["assertion phrased as vague approximate match","multiple unrelated claims hidden in one scenario","shared mutable state leaking between runs"]',
  '["correctness","determinism","clarity"]',
  '[]'
),
(
  'golden-snapshot-comparison',
  'compare current output byte-for-byte against a previously approved reference artifact',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Function","@Media","@Data"]',
  'the scenario <name> is\n  intended to catch silent drift in stable serialised outputs.\n  given the @Media matching "approved reference artifact pinned at known revision" with at-least 0.95 confidence.\n  when the @Function matching "deterministic renderer under test" with at-least 0.9 confidence produces a fresh artifact from fixed inputs.\n  then the fresh artifact is byte-identical to the reference artifact.\n  favor reproducibility.',
  '["rubber-stamped regeneration of reference after every run","nondeterministic fields like timestamps embedded in output","reference drift unreviewed across long periods"]',
  '["reproducibility","correctness","auditability"]',
  '[]'
),
(
  'property-based-shrinking-test',
  'claim holds over a generated input space and counterexamples shrink to a minimal witness',
  '["property"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Function","@Data"]',
  'the property <name> is\n  intended to defend a universal invariant of the subject across a broad input space.\n  generator the @Data matching "structured random input within declared bounds" with at-least 0.9 confidence.\n  uses the @Function matching "subject under property test" with at-least 0.9 confidence.\n  requires each generated input to lie inside the declared domain.\n  ensures any failing input is reducible to a minimal witness that still fails.\n  favor correctness.',
  '["generator too narrow to exercise real edge cases","shrinker loses the failing condition and returns green","claim secretly conditions on the generator distribution"]',
  '["correctness","statistical_rigor","reproducibility"]',
  '[]'
),
(
  'mutation-test-survivor',
  'detect assertions that fail to catch small semantic perturbations of the subject',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Function","@Data"]',
  'the scenario <name> is\n  intended to expose test suites that pass even when the subject is silently altered.\n  given the @Function matching "subject with one small semantic perturbation injected" with at-least 0.9 confidence.\n  when the existing assertion battery runs unchanged against the perturbed subject.\n  then at least one assertion fails and names the perturbation it caught.\n  favor correctness.',
  '["perturbation equivalent to original and therefore uncatchable","flaky assertion masks genuine survival","runtime explosion from unbounded perturbation set"]',
  '["correctness","auditability","clarity"]',
  '[]'
),
(
  'integration-test-fixture',
  'exercise several collaborating components against a controlled shared environment',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Module","@Data"]',
  'the scenario <name> is\n  intended to verify that several real components cooperate over a shared boundary.\n  given the @Data matching "isolated fixture seeded with known records" with at-least 0.9 confidence.\n  when the @Module matching "collaborating component set under test" with at-least 0.9 confidence exchanges messages through the real boundary.\n  then each observable side effect matches the declared expectation exactly.\n  favor correctness.',
  '["fixture shared between unrelated runs","hidden reliance on wall-clock ordering","mocked boundary that hides the integration under test"]',
  '["correctness","reproducibility","auditability"]',
  '[]'
),
(
  'end-to-end-flow-test',
  'walk a full user-visible flow from entry screen through to final outcome',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Screen","@Event","@Function"]',
  'the scenario <name> is\n  intended to confirm a real user path reaches its declared outcome across every layer.\n  given the @Screen matching "entry screen in initial state" with at-least 0.9 confidence.\n  when the user issues the @Event matching "ordered interaction sequence for target flow" with at-least 0.9 confidence.\n  then the final screen shows the declared outcome and every intermediate @Function matching "step handler along the flow" with at-least 0.85 confidence has recorded success.\n  favor correctness.',
  '["flakiness from animation or network timing","cross-test pollution via shared account state","outcome checked only on the last screen, hiding mid-flow defects"]',
  '["correctness","responsiveness","reproducibility"]',
  '[]'
),
(
  'performance-regression-benchmark',
  'detect a statistically meaningful slowdown of a subject against a pinned baseline',
  '["property"]',
  '["intended","generator","uses","requires","ensures","favor"]',
  '["@Function","@Data"]',
  'the property <name> is\n  intended to catch performance loss before it reaches production.\n  generator the @Data matching "representative workload drawn from recorded traffic" with at-least 0.9 confidence.\n  uses the @Function matching "subject whose timing is measured" with at-least 0.9 confidence.\n  requires the measurement harness to pin the workload, the host class, and the warm-up budget.\n  ensures the median latency of the subject stays within the declared tolerance of the pinned baseline.\n  favor performance.',
  '["noisy host masks real regressions","baseline quietly refreshed so slowdowns vanish","warm-up too short so cold-start dominates the sample"]',
  '["performance","latency","reproducibility"]',
  '[]'
),
(
  'fuzz-test-corpus',
  'feed a growing corpus of mutated inputs and flag any crash, hang, or invariant break',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Function","@Data"]',
  'the scenario <name> is\n  intended to harden the subject against adversarial and malformed inputs.\n  given the @Data matching "corpus of seed inputs plus coverage-guided mutations" with at-least 0.9 confidence.\n  when the @Function matching "subject under fuzz harness" with at-least 0.9 confidence consumes each input under a bounded time and memory budget.\n  then no input causes a crash, hang, or declared invariant break, and any finding is minimised and added to the corpus.\n  favor correctness.',
  '["coverage plateau hides unreached branches","oracle too weak to notice silent corruption","minimised finding stored without a pinned reproduction seed"]',
  '["correctness","auditability","reproducibility"]',
  '[]'
),
(
  'chaos-injection-test',
  'inject a realistic environmental fault and confirm the system degrades within declared bounds',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Module","@Event"]',
  'the scenario <name> is\n  intended to verify graceful degradation under a named environmental fault.\n  given the @Module matching "target system under steady-state load" with at-least 0.9 confidence.\n  when the @Event matching "single injected fault drawn from the declared fault catalogue" with at-least 0.9 confidence is applied for a bounded window.\n  then user-visible success rate, latency, and data integrity stay within the declared degradation envelope and the system returns to steady state after the window closes.\n  favor availability.',
  '["injection scope leaks beyond the declared window","degradation envelope drawn loosely enough that any behaviour passes","fault catalogue untested against real historical incidents"]',
  '["availability","correctness","auditability"]',
  '[]'
),
(
  'contract-test-provider-consumer',
  'pin a shared boundary so provider and consumer stay compatible across independent releases',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Module","@Data"]',
  'the scenario <name> is\n  intended to prevent drift across independently released sides of a shared boundary.\n  given the @Data matching "pinned contract document co-owned by both sides" with at-least 0.95 confidence.\n  when the @Module matching "provider implementation under test" with at-least 0.9 confidence replays every interaction declared in the contract and the @Module matching "consumer implementation under test" with at-least 0.9 confidence replays every expectation declared in the contract.\n  then every provider response satisfies the contract shape and every consumer expectation is met by the contract.\n  favor forward_compatibility.',
  '["contract edited by one side without the other noticing","optional fields read as required on the consumer side","contract version not pinned inside the recorded interaction"]',
  '["forward_compatibility","correctness","auditability"]',
  '[]'
);


-- Parallel-seeded batch 3 -- observability + persistence + numerical
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'structured-log-event',
  'emit a structured log record with typed fields for a named operation outcome',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to record a structured observation about <operation> with typed fields.\n  uses the @Function matching "serialize fields to structured record" with at-least 0.9 confidence.\n  requires the @Data matching "bounded key-value map with stable schema" with at-least 0.9 confidence.\n  ensures every record carries timestamp and severity and operation identifier.\n  hazard unbounded field cardinality inflates storage and slows indexing.\n  favor auditability.',
  '["unbounded field cardinality","leaking sensitive values into fields","non-stable schema drift"]',
  '["auditability","clarity"]',
  '[]'
),
(
  'distributed-trace-span',
  'open and close a span that records a unit of work across process boundaries',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to represent <unit_of_work> as a timed span linked to a parent span when present.\n  uses the @Function matching "create span with parent link and attributes" with at-least 0.9 confidence.\n  requires the @Data matching "parent span context or empty root marker" with at-least 0.9 confidence.\n  ensures the span is closed exactly once with a status and end timestamp.\n  hazard forgetting to close a span produces dangling intervals.\n  favor auditability.',
  '["dangling open spans","missing parent link breaks reconstruction","attribute cardinality explosion"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'metric-counter-gauge-histogram',
  'declare a metric instrument with kind counter or gauge or histogram and record samples',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to update a metric with a sample under a bounded label set.\n  uses the @Function matching "update metric instrument by kind" with at-least 0.9 confidence.\n  requires the @Data matching "metric kind is counter or gauge or histogram" with at-least 0.9 confidence.\n  ensures counter samples are non-negative and gauge samples replace prior value and histogram samples fall in a declared bucket layout.\n  hazard high-cardinality label combinations exhaust memory.\n  favor performance.',
  '["high-cardinality labels","mixing kinds on one name","histogram bucket drift across versions"]',
  '["performance","correctness"]',
  '[]'
),
(
  'health-check-probe',
  'expose a probe that reports liveness and readiness of a component',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to answer whether <component> is live and ready to accept work.\n  uses the @Function matching "aggregate dependent check results" with at-least 0.9 confidence.\n  requires the @Data matching "list of dependent check outcomes with timeout" with at-least 0.9 confidence.\n  ensures the probe returns within a fixed deadline and classifies the component as live or not-live and ready or not-ready.\n  hazard a probe that performs heavy work can itself become the outage.\n  favor availability.',
  '["probe performs heavy work","timeout longer than caller deadline","conflating liveness with readiness"]',
  '["availability","responsiveness"]',
  '[]'
),
(
  'error-budget-sli',
  'compute a service level indicator against an error budget over a rolling window',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to measure the fraction of good events over a rolling window and compare against a target.\n  uses the @Function matching "ratio of good events to total events in window" with at-least 0.9 confidence.\n  requires the @Data matching "good event count and total event count and target ratio" with at-least 0.9 confidence.\n  ensures the indicator is reported with the window bounds and the remaining budget.\n  hazard sampling gaps make the indicator silently overstate health.\n  favor statistical_rigor.',
  '["sampling gaps in the window","clock skew across reporters","target drift without versioning"]',
  '["statistical_rigor","auditability"]',
  '[]'
),
(
  'correlation-identifier-propagation',
  'extract and inject a correlation identifier across a request boundary',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to carry a correlation identifier from an incoming request into every outgoing request and log record.\n  uses the @Function matching "extract identifier from carrier and inject into outgoing carrier" with at-least 0.9 confidence.\n  requires the @Data matching "carrier headers with a reserved identifier key" with at-least 0.9 confidence.\n  ensures the identifier is preserved unchanged when present and freshly generated when absent.\n  hazard losing the identifier at an asynchronous boundary fragments the trace.\n  favor correctness.',
  '["loss at asynchronous boundary","identifier rewritten by intermediary","missing generation when absent"]',
  '["correctness","auditability"]',
  '[]'
),
(
  'sampled-trace-export',
  'decide whether to export a trace and batch accepted traces to a sink',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to decide admission of a trace by a sampling policy and forward accepted traces to a sink in batches.\n  uses the @Function matching "apply sampling policy then enqueue for batched export" with at-least 0.9 confidence.\n  requires the @Data matching "sampling policy and bounded export queue" with at-least 0.9 confidence.\n  ensures accepted traces are delivered at most once and rejected traces are counted.\n  hazard unbounded queues collapse the host under export backpressure.\n  favor reproducibility.',
  '["unbounded export queue","sampling policy hides rare failures","duplicate export on retry"]',
  '["reproducibility","performance"]',
  '[]'
),
(
  'runtime-heap-snapshot',
  'capture a heap snapshot for offline inspection under guard conditions',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to write the current heap state of a process to a snapshot artifact.\n  uses the @Function matching "freeze allocator and serialize reachable graph" with at-least 0.9 confidence.\n  requires the @Data matching "snapshot sink with sufficient free space and permission" with at-least 0.9 confidence.\n  ensures the process resumes with the same observable state after the snapshot completes.\n  hazard capture stalls the process for the duration of the freeze.\n  favor reproducibility.',
  '["long freeze stalls the process","snapshot contains sensitive values","sink lacks free space"]',
  '["reproducibility","auditability"]',
  '[]'
),
(
  'slow-query-log',
  'record queries whose duration exceeds a threshold with bounded evidence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to capture a query and its plan when wall-clock duration exceeds a threshold.\n  uses the @Function matching "compare duration to threshold and emit evidence record" with at-least 0.9 confidence.\n  requires the @Data matching "duration threshold and redaction rules for parameters" with at-least 0.9 confidence.\n  ensures parameter values are redacted and the plan fingerprint is stable across invocations.\n  hazard verbatim parameter capture leaks sensitive values.\n  favor auditability.',
  '["verbatim parameter capture","unstable plan fingerprint","threshold too low floods the log"]',
  '["auditability","clarity"]',
  '[]'
),
(
  'alert-routing-rule',
  'route a triggered alert to receivers by matching labels with deduplication and silencing',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to deliver an alert to receivers whose selector matches its labels while respecting silence windows.\n  uses the @Function matching "match labels against rule selectors and deduplicate within a window" with at-least 0.9 confidence.\n  requires the @Data matching "rule set with selectors and silence windows and receiver bindings" with at-least 0.9 confidence.\n  ensures identical alerts within the deduplication window are delivered once and silenced alerts are suppressed with a recorded reason.\n  hazard overly broad silences hide live incidents.\n  favor availability.',
  '["overly broad silences","deduplication window masks recurrence","selector matches everything"]',
  '["availability","auditability"]',
  '[]'
),
(
  'persist-write-ahead-log',
  'append mutations to a durable log before applying them',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to record a mutation in the write-ahead log before state is changed.\n  uses the @Data matching "append-only log segment" with at-least 0.9 confidence.\n  requires the log segment to be flushed to durable storage before acknowledgement.\n  ensures a crash between log write and state apply can be replayed deterministically.\n  hazard unbounded log growth without checkpoint truncation.\n  favor reproducibility.',
  '["fsync skipped under load","log truncation races with replay","partial record on torn write"]',
  '["reproducibility","correctness","auditability"]',
  '[]'
),
(
  'persist-snapshot-plus-incremental-backup',
  'capture a full snapshot then layer incremental deltas on top',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to emit a delta backup relative to the most recent base snapshot.\n  uses the @Data matching "base snapshot manifest" with at-least 0.9 confidence.\n  requires the base snapshot to be immutable and checksummed.\n  ensures a full restore is the base snapshot plus every delta in order.\n  hazard a broken delta chain silently corrupts every later restore.\n  favor reproducibility.',
  '["broken delta chain","base snapshot mutated in place","missing delta ordering"]',
  '["reproducibility","auditability","availability"]',
  '[]'
),
(
  'persist-forward-only-migration',
  'apply a schema migration that has no downgrade path',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to transform the schema from one version to the next.\n  uses the @Data matching "migration script registry" with at-least 0.9 confidence.\n  requires every prior migration to have been applied exactly once.\n  ensures the recorded schema version advances by one on success and stays unchanged on failure.\n  hazard a partially applied migration leaves rows in a shape no version can read.\n  favor correctness.',
  '["partial apply on failure","skipped prior migration","irreversible data loss"]',
  '["correctness","auditability","determinism"]',
  '[]'
),
(
  'persist-versioned-schema-evolution',
  'evolve a schema by adding versioned variants side by side',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","exposes","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to let a reader decode any record written under any past schema version.\n  uses the @Data matching "versioned record envelope" with at-least 0.9 confidence.\n  composes the @Function matching "decode record at version" with at-least 0.9 confidence.\n  requires every stored record to carry its originating schema version.\n  ensures no past version is silently dropped without an explicit retirement step.\n  hazard version sprawl makes the decoder surface grow without bound.\n  exposes a decoder that selects the reader by embedded version tag.\n  favor forward_compatibility.',
  '["version sprawl","missing version tag on legacy rows","decoder fan-out"]',
  '["forward_compatibility","correctness","auditability"]',
  '[]'
),
(
  'persist-indexed-lookup-table',
  'maintain a secondary index so lookups avoid a full scan',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to resolve a key to a row identifier without scanning the base table.\n  exposes a read path that is logarithmic in the row count.\n  exposes a write path that updates the index in the same transaction as the base row.\n  favor latency.',
  '["index drifts from base table on crash","write amplification","stale index after bulk load"]',
  '["latency","performance","correctness"]',
  '[]'
),
(
  'persist-soft-delete-with-tombstone',
  'mark a row deleted with a tombstone instead of removing it',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to mark a row as deleted by writing a tombstone with the deletion time.\n  uses the @Data matching "tombstone marker" with at-least 0.9 confidence.\n  requires every reader to filter out rows carrying a tombstone.\n  ensures the row can be audited and resurrected until a compaction sweep removes it.\n  hazard a reader that forgets the tombstone filter returns deleted rows as live.\n  favor auditability.',
  '["reader forgets tombstone filter","tombstone never compacted","resurrection after compaction"]',
  '["auditability","correctness","reproducibility"]',
  '[]'
),
(
  'persist-time-series-retention-policy',
  'drop time series samples older than a bounded retention window',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to remove every sample whose timestamp is older than a retention cutoff.\n  uses the @Data matching "time series shard index" with at-least 0.9 confidence.\n  requires the retention cutoff to be monotonic and never move backward.\n  ensures storage used by expired samples is released within one retention sweep.\n  hazard a clock skew expires samples earlier than the stated policy.\n  favor determinism.',
  '["clock skew","retention moves backward","unbounded growth on sweep failure"]',
  '["determinism","reproducibility","auditability"]',
  '[]'
),
(
  'persist-bloom-filter-membership-test',
  'use a compact probabilistic structure to skip definite misses',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to report whether a key might be in a set so a definite miss skips the disk read.\n  uses the @Data matching "bit array with hash functions" with at-least 0.9 confidence.\n  requires the caller to treat a positive result as unconfirmed and verify against the authoritative store.\n  ensures a false negative is impossible while a false positive is bounded by the configured rate.\n  hazard callers that trust a positive as definite produce wrong answers.\n  favor performance.',
  '["false positive treated as definite","filter never resized as set grows","hash seed drift"]',
  '["performance","latency","correctness"]',
  '[]'
),
(
  'persist-copy-on-write-versioning',
  'produce a new version by writing changed pages and sharing the rest',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","exposes","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to expose each version as an immutable snapshot that shares unchanged pages with its parent.\n  uses the @Data matching "page reference graph" with at-least 0.9 confidence.\n  composes the @Function matching "write page and rebind parent pointer" with at-least 0.9 confidence.\n  requires every write to allocate a fresh page and never mutate a shared one.\n  ensures any past version is readable as long as its root pointer is retained.\n  hazard orphaned pages accumulate if reference counts are not maintained.\n  exposes a root pointer per version and a reachability sweep for reclamation.\n  favor reproducibility.',
  '["orphaned page leak","shared page mutated by mistake","reference count underflow"]',
  '["reproducibility","correctness","auditability"]',
  '[]'
),
(
  'persist-materialized-view-refresh',
  'recompute a precomputed view so reads stay within a freshness bound',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to recompute a view from base tables whenever its staleness exceeds a freshness bound.\n  uses the @Data matching "materialized view definition" with at-least 0.9 confidence.\n  requires the refresh to observe a consistent snapshot of every base table.\n  ensures a reader sees either the prior committed view or the new one, never a partial blend.\n  hazard a long refresh overlaps the next scheduled refresh and starves readers.\n  favor latency.',
  '["overlapping refresh","partial view visible to readers","base table snapshot skew"]',
  '["latency","performance","correctness"]',
  '[]'
),
(
  'fixed-point-iteration',
  'iterate a contraction map to convergence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to apply a step repeatedly to a seed until the change drops below a tolerance.\n  uses the @Function matching "contraction-step" with at-least 0.9 confidence.\n  requires the step to be a contraction on the working domain.\n  ensures the returned value satisfies the tolerance bound on its own update.\n  hazard divergence when the map is not contractive.\n  favor numerical_stability.',
  '["non-contractive map","oscillation","slow convergence"]',
  '["numerical_stability","determinism"]',
  '[]'
),
(
  'matrix-vector-product',
  'multiply a dense matrix by a vector',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to compute a matrix applied to a vector producing a new vector.\n  uses the @Data matching "row-major matrix" with at-least 0.9 confidence.\n  requires the matrix column count to equal the vector length.\n  ensures the result length equals the matrix row count.\n  hazard catastrophic cancellation on nearly-opposite summands.\n  favor numerical_stability.',
  '["shape mismatch","cancellation","overflow"]',
  '["numerical_stability","performance"]',
  '[]'
),
(
  'ordinary-differential-equation-integrator',
  'advance an initial-value problem by one adaptive step',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to advance state under a derivative by a step with local error under tolerance.\n  uses the @Function matching "embedded pair step" with at-least 0.9 confidence.\n  requires the derivative to be evaluable at every probe point in the trial step.\n  ensures the returned state carries an estimated local error within the declared tolerance.\n  hazard stiffness causing vanishing accepted step sizes.\n  favor numerical_stability.',
  '["stiffness","step size underflow","error estimate bias"]',
  '["numerical_stability","correctness"]',
  '[]'
),
(
  'monte-carlo-estimator',
  'estimate an expectation by independent sampling',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to approximate the mean of an integrand under a sampler using a fixed sample count.\n  uses the @Function matching "independent sampler" with at-least 0.9 confidence.\n  requires the sampler draws to be independent and identically distributed.\n  ensures the reported estimate carries a standard error decreasing like one over square-root of sample count.\n  hazard heavy-tailed integrand inflating variance beyond the reported error.\n  favor statistical_rigor.',
  '["heavy tails","correlated draws","seed reuse"]',
  '["statistical_rigor","reproducibility"]',
  '[]'
),
(
  'gradient-descent-step',
  'update parameters along the negative gradient',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to move parameters by a learning rate times the negative of the gradient.\n  uses the @Function matching "loss gradient" with at-least 0.9 confidence.\n  requires the gradient to have the same shape as the parameters.\n  ensures the returned parameters equal the prior parameters minus the learning rate times the gradient componentwise.\n  hazard learning rate too large causing loss to increase.\n  favor numerical_stability.',
  '["exploding step","vanishing step","shape mismatch"]',
  '["numerical_stability","determinism"]',
  '[]'
),
(
  'nearest-neighbor-index',
  'return the closest stored points to a query',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to return the k closest entries in an index to a query under a distance.\n  uses the @Data matching "spatial partition index" with at-least 0.9 confidence.\n  requires the query to share dimensionality with entries in the index.\n  ensures the returned list holds exactly k entries ordered by non-decreasing distance.\n  hazard distance ties producing nondeterministic ordering.\n  favor determinism.',
  '["tie breaking","dimensionality mismatch","stale index"]',
  '["determinism","performance"]',
  '[]'
),
(
  'basis-decomposition-transform',
  'project a signal onto an orthogonal basis',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to decompose a signal into coefficients against an orthogonal basis.\n  uses the @Data matching "orthogonal basis" with at-least 0.9 confidence.\n  requires the basis vectors to be mutually orthogonal within a tolerance.\n  ensures reconstructing from the returned coefficients recovers the signal within the tolerance.\n  hazard non-orthogonal basis producing leakage across coefficients.\n  favor numerical_stability.',
  '["non orthogonality","aliasing","boundary artifacts"]',
  '["numerical_stability","correctness"]',
  '[]'
),
(
  'numerical-integration-rule',
  'approximate a definite integral by a quadrature rule',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to approximate the integral of an integrand over an interval using a quadrature rule.\n  uses the @Function matching "quadrature node weight set" with at-least 0.9 confidence.\n  requires the integrand to be finite at every node selected by the rule.\n  ensures the reported estimate carries an error bound consistent with the rule order on smooth integrands.\n  hazard endpoint singularity invalidating the rule error bound.\n  favor numerical_stability.',
  '["endpoint singularity","oscillatory integrand","node evaluation failure"]',
  '["numerical_stability","correctness"]',
  '[]'
),
(
  'confidence-interval-estimator',
  'build a confidence interval for a population parameter',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to bracket the parameter of a sample at a nominal coverage using a declared method.\n  uses the @Function matching "interval construction method" with at-least 0.9 confidence.\n  requires the sample to satisfy the independence assumptions demanded by the method.\n  ensures the returned interval reports its coverage method and sample size alongside the bounds.\n  hazard dependent observations inflating the true miscoverage beyond the nominal rate.\n  favor statistical_rigor.',
  '["dependent samples","small sample bias","method assumption violation"]',
  '["statistical_rigor","auditability"]',
  '[]'
),
(
  'discrete-event-simulator',
  'advance a simulation through time-ordered events',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to process events from an event queue against a state until a stop condition holds.\n  uses the @Data matching "priority ordered event queue" with at-least 0.9 confidence.\n  requires every event in the queue to carry a scheduled time greater than or equal to the current simulation clock.\n  ensures events are handled in non-decreasing order of scheduled time with deterministic tie-breaking.\n  hazard simultaneous events ordered by insertion instead of a declared tie rule.\n  favor determinism.',
  '["tie ordering","clock regression","unbounded event storm"]',
  '["determinism","reproducibility"]',
  '[]'
);


-- Parallel-seeded batch 4 -- build + networking + graphics
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'continuous-integration-pipeline',
  'run validation stages on every change before merge',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data","@Event"]',
  'the function <name> is\n  intended to validate every change across build, test, and analysis stages before merge.\n  uses the @Data matching "source revision" with at-least 0.9 confidence.\n  uses the @Event matching "change proposed" with at-least 0.9 confidence.\n  requires every stage to report a terminal pass or fail status.\n  ensures a failing stage blocks promotion of the source revision.\n  hazard flaky stages erode trust and invite bypass.\n  favor reproducibility.',
  '["flaky stage","bypass on red"]',
  '["reproducibility","auditability","determinism"]',
  '[]'
),
(
  'artifact-promotion-gate',
  'promote a built artifact between stages only when gate conditions hold',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to advance an artifact from a lower stage to a higher stage only when gate conditions hold.\n  uses the @Data matching "artifact manifest" with at-least 0.9 confidence.\n  requires every gate condition to evaluate to pass before advancement.\n  ensures the artifact at the higher stage is byte-identical to the artifact at the lower stage.\n  hazard rebuilding between stages silently changes the artifact.\n  favor auditability.',
  '["rebuild between stages","skipped gate"]',
  '["auditability","reproducibility","correctness"]',
  '[]'
),
(
  'blue-green-deployment',
  'switch traffic atomically between two full environments',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data","@Event"]',
  'the function <name> is\n  intended to move live traffic from the active environment to the standby environment in a single step.\n  uses the @Data matching "environment descriptor" with at-least 0.9 confidence.\n  uses the @Event matching "switch requested" with at-least 0.9 confidence.\n  requires the standby environment to have passed health checks before the switch.\n  ensures at most one environment receives live traffic at any instant.\n  hazard stale session state in the standby environment drops in-flight work.\n  favor availability.',
  '["stale standby","split traffic"]',
  '["availability","determinism","correctness"]',
  '[]'
),
(
  'canary-rollout-fraction',
  'expose a new release to a bounded fraction of traffic first',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to send a bounded fraction of traffic to a candidate release while the rest stays on the baseline release.\n  uses the @Data matching "release descriptor" with at-least 0.9 confidence.\n  requires the canary fraction to lie within zero and one inclusive.\n  ensures exactly the declared fraction of requests reach the candidate release within measurement tolerance.\n  hazard too-small a canary hides regressions that surface only at full scale.\n  favor availability.',
  '["undersized canary","skewed sampling"]',
  '["availability","statistical_rigor","auditability"]',
  '[]'
),
(
  'rollback-to-previous-release',
  'restore the immediately prior release when the current one misbehaves',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data","@Event"]',
  'the function <name> is\n  intended to restore a previous release when the current release is observed to misbehave.\n  uses the @Data matching "release ledger" with at-least 0.9 confidence.\n  uses the @Event matching "rollback requested" with at-least 0.9 confidence.\n  requires the previous release to remain retrievable and runnable.\n  ensures live traffic returns to the previous release before the rollback is acknowledged as resolved.\n  hazard irreversible data migrations block rollback.\n  favor availability.',
  '["irreversible migration","lost previous release"]',
  '["availability","auditability","reproducibility"]',
  '[]'
),
(
  'immutable-build-tag',
  'bind a content-addressed tag to a build that never moves',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to bind a build tag to a build artifact such that the binding never moves.\n  uses the @Data matching "build artifact" with at-least 0.9 confidence.\n  requires the build tag to be derived from the content of the build artifact.\n  ensures resolving the tag yields the same artifact for all time.\n  hazard reusing a tag for a new artifact invalidates every prior reference.\n  favor reproducibility.',
  '["tag reuse","mutable tag"]',
  '["reproducibility","auditability","determinism"]',
  '[]'
),
(
  'dependency-lockfile',
  'pin every transitive dependency to exact resolved versions',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to pin every direct and transitive dependency of a project to an exact resolved version.\n  exposes a resolved version for each dependency name reachable from the project.\n  favor reproducibility.',
  '["partial lock","drift on resolve"]',
  '["reproducibility","determinism","auditability"]',
  '[]'
),
(
  'staged-release-channel',
  'route releases through ordered maturity channels',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to place a release on a channel within an ordered sequence from least to most stable.\n  uses the @Data matching "channel registry" with at-least 0.9 confidence.\n  requires the release to have occupied every earlier channel before the current channel.\n  ensures subscribers of the channel see only releases that have cleared all earlier channels.\n  hazard skipping a channel exposes subscribers to unverified releases.\n  favor auditability.',
  '["skipped channel","out-of-order publish"]',
  '["auditability","reproducibility","availability"]',
  '[]'
),
(
  'deploy-preflight-check',
  'verify environment and artifact compatibility before deploying',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to verify that a target environment can accept a candidate artifact before deployment begins.\n  uses the @Data matching "environment descriptor" with at-least 0.9 confidence.\n  requires every compatibility rule between the candidate artifact and the target environment to hold.\n  ensures deployment proceeds only when all preflight checks pass.\n  hazard preflight that drifts from the real deploy step gives false confidence.\n  favor correctness.',
  '["drifted preflight","partial check"]',
  '["correctness","auditability","determinism"]',
  '[]'
),
(
  'release-note-changelog',
  'record a human-readable summary of changes per release',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to record for a release a human-readable summary of what changed since the previous release.\n  exposes a change summary and affected area for each change included in the release.\n  favor documentation.',
  '["missing entry","vague summary"]',
  '["documentation","auditability","discoverability"]',
  '[]'
),
(
  'connection-pool-lifecycle',
  'maintain a bounded pool of reusable peer sessions with health tracking',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to reuse peer sessions across many short requests.\n  uses the @Data matching "session handle" with at-least 0.9 confidence.\n  composes the @Function matching "acquire-release-evict" with at-least 0.9 confidence.\n  requires bounded capacity and per-session idle timeout.\n  ensures every acquired session is either returned or evicted.\n  hazard session leak when caller drops without release.\n  favor availability.',
  '["session leak when caller drops without release","pool starvation under burst load"]',
  '["availability","performance","determinism"]',
  '[]'
),
(
  'request-timeout-with-deadline',
  'enforce an absolute deadline across a whole request rather than per-hop timeouts',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to bound the total time a request may consume end-to-end.\n  uses the @Data matching "deadline instant" with at-least 0.9 confidence.\n  requires the deadline to be propagated to every downstream call.\n  ensures the caller observes failure no later than the deadline.\n  hazard per-hop timeouts summing past the deadline when propagation is forgotten.\n  favor latency.',
  '["per-hop timeouts summing past the deadline when propagation is forgotten"]',
  '["latency","availability","determinism"]',
  '[]'
),
(
  'keepalive-heartbeat',
  'detect dead peers on idle sessions by exchanging periodic liveness probes',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Event","@Function"]',
  'the concept <name> is\n  intended to detect silently broken sessions that carry no traffic.\n  uses the @Event matching "probe-pong cycle" with at-least 0.9 confidence.\n  composes the @Function matching "send-probe-and-on-pong handler" with at-least 0.9 confidence.\n  requires probe interval shorter than any intermediary idle cutoff.\n  ensures a missed pong within the window marks the session dead.\n  hazard heartbeat storms on large pools amplifying load.\n  favor availability.',
  '["heartbeat storms on large pools amplifying load"]',
  '["availability","responsiveness"]',
  '[]'
),
(
  'backoff-with-jitter',
  'space retry attempts with randomized exponential delays to avoid synchronized bursts',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to produce the next retry delay after a transient failure.\n  uses the @Data matching "attempt count and base interval" with at-least 0.9 confidence.\n  requires a cap that bounds the maximum delay.\n  ensures two peers retrying the same failure do not align in time.\n  hazard tight loops when jitter is omitted and many peers retry at once.\n  favor availability.',
  '["tight loops when jitter is omitted and many peers retry at once"]',
  '["availability","determinism"]',
  '[]'
),
(
  'message-framing-length-prefixed',
  'carry a sequence of messages over a byte stream by prefixing each with its length',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to recover discrete messages from an ordered byte stream.\n  uses the @Data matching "length header plus payload" with at-least 0.9 confidence.\n  composes the @Function matching "encode-decode frame" with at-least 0.9 confidence.\n  requires a fixed maximum frame size rejected early.\n  ensures decoder resynchronizes only at a valid frame boundary.\n  hazard oversized length header exhausting memory when no cap is enforced.\n  favor correctness.',
  '["oversized length header exhausting memory when no cap is enforced"]',
  '["correctness","determinism","performance"]',
  '[]'
),
(
  'protocol-version-negotiation',
  'agree on a shared protocol version at the start of a session',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to select the highest version both peers can speak.\n  uses the @Data matching "version offer set" with at-least 0.9 confidence.\n  requires a total order on version identifiers.\n  ensures both peers commit to the same version before any payload is exchanged.\n  hazard downgrade when an attacker strips higher versions from the offer.\n  favor correctness.',
  '["downgrade when an attacker strips higher versions from the offer"]',
  '["correctness","forward_compatibility","auditability"]',
  '[]'
),
(
  'bidirectional-streaming-channel',
  'carry independent message streams in both directions over one session',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Event","@Function"]',
  'the concept <name> is\n  intended to allow either peer to send messages without waiting for the other.\n  uses the @Event matching "close event" with at-least 0.9 confidence.\n  composes the @Function matching "send-receive-half-close" with at-least 0.9 confidence.\n  requires per-direction flow control with bounded buffers.\n  ensures closing one direction leaves the other direction drainable.\n  hazard deadlock when both peers block on full send buffers.\n  favor responsiveness.',
  '["deadlock when both peers block on full send buffers"]',
  '["responsiveness","availability","performance"]',
  '[]'
),
(
  'load-balanced-upstream-set',
  'spread requests across a set of equivalent upstreams with health-aware selection',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to route each request to a healthy member of a replica group.\n  uses the @Data matching "member list with health flag" with at-least 0.9 confidence.\n  composes the @Function matching "pick-mark-rehabilitate" with at-least 0.9 confidence.\n  requires a passive or active signal that classifies members as healthy.\n  ensures an unhealthy member stops receiving new requests within a bounded delay.\n  hazard herding when every client picks the same member after a failover.\n  favor availability.',
  '["herding when every client picks the same member after a failover"]',
  '["availability","latency","responsiveness"]',
  '[]'
),
(
  'transport-encryption-handshake',
  'establish an authenticated encrypted channel before any payload is exchanged',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to prove peer identity and derive session keys before payload traffic.\n  uses the @Data matching "peer identity and shared secret" with at-least 0.9 confidence.\n  composes the @Function matching "exchange-verify-derive" with at-least 0.9 confidence.\n  requires identity verification completed before any encrypted payload is sent.\n  ensures a successful handshake yields forward-secret keys distinct per session.\n  hazard silent acceptance of an unverified identity leaking session contents.\n  favor auditability.',
  '["silent acceptance of an unverified identity leaking session contents"]',
  '["auditability","correctness","forward_compatibility"]',
  '[]'
),
(
  'graceful-connection-shutdown',
  'drain in-flight work before closing a session rather than cutting it abruptly',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Event"]',
  'the function <name> is\n  intended to close a session after letting outstanding requests finish.\n  uses the @Event matching "close event with drain deadline" with at-least 0.9 confidence.\n  requires new requests rejected immediately once shutdown begins.\n  ensures either all inflight requests complete or the drain deadline fires.\n  hazard indefinite hang when no drain deadline is enforced.\n  favor availability.',
  '["indefinite hang when no drain deadline is enforced"]',
  '["availability","responsiveness","determinism"]',
  '[]'
),
(
  'vector-path-render',
  'rasterize a vector path into pixel coverage with anti-aliased edges',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to rasterize a vector path into a surface with coverage-based anti-aliasing.\n  uses the @Data matching "path with transform and coverage kernel" with at-least 0.9 confidence.\n  favor clarity.',
  '["subpixel seams between adjacent fills","winding-rule ambiguity on self-intersecting paths"]',
  '["clarity","determinism"]',
  '[]'
),
(
  'raster-image-composition',
  'composite layered raster images under a blend mode into one output',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to composite layers under a blend mode into an output surface.\n  uses the @Data matching "layer stack with alpha model" with at-least 0.9 confidence.\n  favor correctness.',
  '["premultiplied vs straight alpha mismatch","gamma-space blending darkening mid-tones"]',
  '["correctness","reproducibility"]',
  '[]'
),
(
  'typeset-glyph-layout',
  'lay out a run of glyphs along a baseline with shaping and kerning',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to lay out glyphs along a baseline with shaping and kerning.\n  uses the @Data matching "font face with shaping and kerning tables" with at-least 0.9 confidence.\n  favor accessibility.',
  '["missing glyph fallback not applied","baseline misalignment across mixed scripts"]',
  '["accessibility","clarity"]',
  '[]'
),
(
  'color-palette-swatch',
  'define a named palette of colors with contrast-aware role assignments',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data"]',
  'the media <name> is\n  intended to define a named set of colors with contrast-aware role assignments.\n  uses the @Data matching "swatches and role map with contrast threshold" with at-least 0.9 confidence.\n  favor accessibility.',
  '["insufficient contrast for foreground-on-background pairs","role collisions when roles outnumber swatches"]',
  '["accessibility","clarity"]',
  '[]'
),
(
  'shader-pipeline-stage',
  'describe a single programmable stage that transforms inputs into outputs per-fragment',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to transform stage inputs into stage outputs per fragment.\n  uses the @Data matching "uniforms and sampler bindings" with at-least 0.9 confidence.\n  favor determinism.',
  '["undefined behavior on division by zero in fragment math","precision loss at low-precision numeric types"]',
  '["determinism","numerical_stability"]',
  '[]'
),
(
  'mesh-vertex-index-buffer',
  'store a triangulated mesh as a vertex buffer paired with an index buffer',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data"]',
  'the media <name> is\n  intended to store vertices paired with indices describing a triangulation.\n  uses the @Data matching "vertex attributes and primitive topology" with at-least 0.9 confidence.\n  favor performance.',
  '["index out of vertex range","degenerate triangles with zero area"]',
  '["performance","correctness"]',
  '[]'
),
(
  'animation-keyframe-interpolation',
  'interpolate a property between keyframes along a timeline using an easing curve',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to interpolate a property between keyframes along a timeline using an easing curve.\n  uses the @Data matching "keyframes with easing and sample rate" with at-least 0.9 confidence.\n  favor responsiveness.',
  '["easing overshoot producing out-of-range values","aliasing when sample rate is below keyframe density"]',
  '["responsiveness","determinism"]',
  '[]'
),
(
  'tiled-image-pyramid',
  'represent a large image as a pyramid of tiled mip levels for bounded-memory access',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data"]',
  'the media <name> is\n  intended to represent a source image as tiled mip levels for bounded-memory access.\n  uses the @Data matching "tile size and level count with downsample filter" with at-least 0.9 confidence.\n  favor latency.',
  '["seams at tile borders from non-overlapping filters","excessive level count inflating storage"]',
  '["latency","performance"]',
  '[]'
),
(
  'text-line-bidi-reorder',
  'reorder a mixed-direction character sequence into visual order for a single line',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to reorder logical text into visual order respecting direction runs.\n  uses the @Data matching "direction runs and mirrored bracket pairs" with at-least 0.9 confidence.\n  favor accessibility.',
  '["mirrored bracket pairs not flipped","neutral characters resolved to the wrong run"]',
  '["accessibility","correctness"]',
  '[]'
),
(
  'geometric-clip-region',
  'restrict rendering to the intersection of a stack of geometric regions',
  '["media"]',
  '["intended","uses","favor"]',
  '["@Data","@Function"]',
  'the media <name> is\n  intended to restrict rendering to the intersection of a region stack.\n  uses the @Data matching "region stack with fill rule and transform" with at-least 0.9 confidence.\n  favor correctness.',
  '["empty intersection silently producing no output","floating-point drift at region boundaries"]',
  '["correctness","determinism"]',
  '[]'
);


-- Parallel-seeded batch 5 -- audio + business + compiler
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'audio-sample-buffer',
  'hold a contiguous window of audio samples for streaming and processing',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to hold a fixed-capacity ring of audio samples shared between producer and consumer.\n  exposes a channel-interleaved frame slice for read access.\n  favor latency.',
  '["wrap-around overwrites unread frames when consumer stalls","tearing when read and write indices cross without atomic publish"]',
  '["latency","determinism"]',
  '[]'
),
(
  'fixed-time-step-synthesizer',
  'render audio in fixed-size frame blocks at a deterministic cadence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to compute the next block of synthesized samples at a fixed step.\n  uses the @Data matching "voice state and modulation snapshot" with at-least 0.9 confidence.\n  requires the @Event matching "block boundary tick" with at-least 0.95 confidence.\n  ensures every step yields exactly one block of equal frame count.\n  hazard control changes applied mid-block produce zipper artifacts.\n  favor determinism.',
  '["denormal sample values stall the inner loop","control updates aliased to block rate cause audible stepping"]',
  '["determinism","latency","numerical_stability"]',
  '[]'
),
(
  'frequency-domain-transform',
  'convert a windowed time-domain block into spectral coefficients',
  '["function"]',
  '["intended","uses","requires","ensures","favor"]',
  '["@Data","@Function"]',
  'the function <name> is\n  intended to project a windowed sample block into a complex spectrum.\n  uses the @Data matching "windowed real sample block of power-of-two length" with at-least 0.95 confidence.\n  requires the @Function matching "analysis window taper" with at-least 0.9 confidence.\n  ensures output bin count matches half the block length plus one for real input.\n  favor numerical_stability.',
  '["spectral leakage when window taper is omitted","loss of precision accumulating across long block sizes in single precision"]',
  '["numerical_stability","correctness","performance"]',
  '[]'
),
(
  'resampling-rate-converter',
  'convert a stream from one sample rate to another while preserving spectral content',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to map an input frame stream at one rate to an output stream at another rate.\n  uses the @Function matching "polyphase low-pass kernel" with at-least 0.9 confidence.\n  requires the @Data matching "rational input-to-output rate ratio" with at-least 0.95 confidence.\n  ensures output band-limit stays below the lower of the two Nyquist limits.\n  hazard aliasing folds back into audible band when low-pass cutoff is set above target Nyquist.\n  favor numerical_stability.',
  '["aliasing from insufficient stop-band attenuation","group delay drift across long streams when fractional phase is not tracked"]',
  '["numerical_stability","correctness","latency"]',
  '[]'
),
(
  'loudness-normalization-pass',
  'adjust gain so a signal matches a target perceived loudness',
  '["function"]',
  '["intended","uses","requires","ensures","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to scale a signal so its integrated perceived loudness meets a target level.\n  uses the @Function matching "perceptual loudness meter" with at-least 0.9 confidence.\n  requires the @Data matching "target loudness in loudness units" with at-least 0.95 confidence.\n  ensures the post-gain measurement is within tolerance of the target.\n  favor reproducibility.',
  '["true-peak overshoot after gain even when integrated loudness is correct","short transients dominate measurement on very brief inputs"]',
  '["reproducibility","correctness","auditability"]',
  '[]'
),
(
  'lossy-compression-codec',
  'encode an audio stream into a smaller representation by discarding inaudible content',
  '["concept"]',
  '["intended","uses","composes","ensures","hazard","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to compress an audio stream by removing components masked by the perceptual model.\n  uses the @Function matching "psychoacoustic masking model" with at-least 0.9 confidence.\n  composes the @Function matching "frequency-domain transform" with at-least 0.9 confidence.\n  ensures decoded output bit-rate stays within the configured budget.\n  hazard repeated encode-decode cycles compound spectral hole artifacts.\n  favor performance.',
  '["pre-echo on sharp transients near block boundaries","tandem coding loss when chained with another lossy stage"]',
  '["performance","portability","correctness"]',
  '[]'
),
(
  'real-time-audio-mixer',
  'sum multiple input streams into a shared output bus within a real-time deadline',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to sum gain-scaled input streams into one output bus per audio callback.\n  uses the @Data matching "per-channel gain envelope" with at-least 0.9 confidence.\n  requires the @Event matching "audio callback deadline" with at-least 0.95 confidence.\n  ensures the callback returns before the deadline for every block.\n  hazard lock acquisition or memory allocation inside the callback can miss the deadline.\n  favor latency.',
  '["priority inversion when worker thread holds a lock the callback waits on","clipping when summed buses exceed full-scale without headroom"]',
  '["latency","determinism","responsiveness"]',
  '[]'
),
(
  'envelope-generator-adsr',
  'shape a control signal through attack, decay, sustain, and release segments',
  '["function"]',
  '["intended","uses","requires","ensures","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to produce a per-sample amplitude curve following four ordered segments.\n  uses the @Data matching "segment durations and sustain level" with at-least 0.95 confidence.\n  requires the @Event matching "note-on and note-off triggers" with at-least 0.95 confidence.\n  ensures the curve is continuous across segment transitions.\n  favor determinism.',
  '["click on retrigger when current level is not used as the new attack start","floating-point drift causes sustain level to wander on long held notes"]',
  '["determinism","numerical_stability","responsiveness"]',
  '[]'
),
(
  'spatial-audio-panning',
  'position a mono source within a multi-channel sound field by computing per-channel gains',
  '["function"]',
  '["intended","uses","requires","ensures","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to derive per-output-channel gains placing a source at a target position.\n  uses the @Data matching "listener position and orientation" with at-least 0.9 confidence.\n  requires the @Data matching "speaker layout description" with at-least 0.95 confidence.\n  ensures the sum of squared per-channel gains is preserved across positions.\n  favor numerical_stability.',
  '["amplitude dip at the midpoint when linear pan law is used instead of equal-power","phantom image collapse when listener leaves the sweet spot"]',
  '["numerical_stability","accessibility","correctness"]',
  '[]'
),
(
  'audio-playback-latency-budget',
  'bound the end-to-end delay from sample submission to audible output',
  '["scenario"]',
  '["intended","given","when","then","favor"]',
  '["@Data","@Event"]',
  'the scenario <name> is\n  intended to verify the path from buffer submission to driver hand-off stays under a stated bound.\n  given the @Data matching "configured block size and sample rate" with at-least 0.95 confidence.\n  when the @Event matching "buffer submitted to output stage" with at-least 0.95 confidence.\n  then the elapsed time to driver hand-off is below the budgeted threshold for every measured block.\n  favor latency.',
  '["measurement skew when timestamps come from different clocks","budget met on average but violated at the tail percentile"]',
  '["latency","responsiveness","reproducibility"]',
  '[]'
),
(
  'shopping-cart-line-item',
  'represent a single line entry in a shopping cart with quantity and unit price',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to record one purchasable unit within a cart at the moment of selection.\n  exposes quantity as integer.\n  exposes unit_price as real.\n  favor auditability.',
  '["non-positive quantity collapses subtotal","unit price drift between cart and checkout"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'order-state-machine',
  'transition an order through its allowed lifecycle states',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to advance an order from one state to the next allowed state.\n  uses the @Data matching "order state" with at-least 0.9 confidence.\n  requires the source state and the target state to form a permitted edge.\n  ensures every accepted transition emits the @Event matching "order state changed" with at-least 0.9 confidence.\n  hazard skipping intermediate states erases audit trail.\n  favor determinism.',
  '["illegal transition","skipped state","duplicate event emission"]',
  '["determinism","auditability","correctness"]',
  '[]'
),
(
  'invoice-line-tax-calculation',
  'compute tax for one invoice line given jurisdiction and rate',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to derive the tax amount owed on a single invoice line.\n  uses the @Data matching "tax rate table" with at-least 0.9 confidence.\n  requires the line subtotal and the jurisdiction code to be present.\n  ensures the returned amount is rounded by the jurisdiction rounding rule.\n  hazard floating-point drift in cumulative tax across many lines.\n  favor numerical_stability.',
  '["rounding mode mismatch","missing jurisdiction","compounded float error"]',
  '["numerical_stability","auditability","correctness"]',
  '[]'
),
(
  'inventory-stock-ledger',
  'append-only ledger of stock movements per warehouse and item',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","exposes","favor"]',
  '["@Data","@Event"]',
  'the concept <name> is\n  intended to record every increment and decrement of on-hand quantity as immutable rows.\n  uses the @Data matching "stock movement" with at-least 0.9 confidence.\n  composes the @Event matching "stock adjusted" with at-least 0.9 confidence.\n  requires every movement to carry a signed quantity and a source reference.\n  ensures the running balance for a warehouse-item pair equals the sum of its movements.\n  hazard mutating a past row breaks reconciliation.\n  exposes a current balance view.\n  favor auditability.',
  '["row mutation","missing source ref","negative balance without backorder flag"]',
  '["auditability","reproducibility","correctness"]',
  '[]'
),
(
  'subscription-billing-cycle',
  'generate the next billing period for an active subscription',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to produce the next period boundary and charge entry for a subscription.\n  uses the @Data matching "subscription plan" with at-least 0.9 confidence.\n  requires the previous period end and the plan interval to be known.\n  ensures the new period start equals the previous period end and emits the @Event matching "cycle advanced" with at-least 0.9 confidence.\n  hazard double-advancing on retry produces duplicate charges.\n  favor determinism.',
  '["double advance on retry","clock skew at period boundary","timezone drift"]',
  '["determinism","auditability","correctness"]',
  '[]'
),
(
  'refund-with-reason',
  'issue a refund against a prior charge with a recorded reason code',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to reverse part or all of a prior charge while recording why.\n  uses the @Data matching "charge record" with at-least 0.9 confidence.\n  requires the refund amount to be at most the unrefunded balance of the charge.\n  ensures a refund row is written and the @Event matching "refund issued" with at-least 0.9 confidence is emitted.\n  hazard refunding more than the charge through concurrent partial refunds.\n  favor auditability.',
  '["over-refund race","missing reason code","orphan refund without charge"]',
  '["auditability","correctness","determinism"]',
  '[]'
),
(
  'double-entry-journal',
  'post a balanced journal entry across two or more accounts',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to commit a set of debits and credits as one atomic accounting transaction.\n  uses the @Data matching "chart of accounts" with at-least 0.9 confidence.\n  requires the sum of debits to equal the sum of credits in the entry currency.\n  ensures the entry is rejected when the sums diverge by more than the rounding tolerance.\n  hazard partial posting leaves the ledger out of balance.\n  favor auditability.',
  '["unbalanced entry","partial commit","mixed currency without conversion"]',
  '["auditability","correctness","determinism"]',
  '[]'
),
(
  'approval-workflow-reviewer',
  'route a request through ordered reviewers until approved or rejected',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","exposes","favor"]',
  '["@Function","@Event"]',
  'the concept <name> is\n  intended to drive a request through a sequence of reviewers each holding veto power.\n  uses the @Function matching "assign next reviewer" with at-least 0.9 confidence.\n  composes the @Event matching "decision recorded" with at-least 0.9 confidence.\n  requires the reviewer order and the quorum rule to be defined before routing begins.\n  ensures the request reaches a terminal state of approved or rejected within the configured deadline.\n  hazard a reviewer acting outside their delegated scope.\n  exposes a workflow completed event.\n  favor auditability.',
  '["scope violation","stalled queue","self-approval"]',
  '["auditability","correctness","accessibility"]',
  '[]'
),
(
  'price-currency-conversion',
  'convert a price from one currency to another at a stated rate and time',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to restate an amount in a target currency using a dated exchange rate.\n  uses the @Data matching "exchange rate snapshot" with at-least 0.9 confidence.\n  requires the source currency, the target currency, and the rate timestamp to be present.\n  ensures the result records both the rate value and the rate timestamp alongside the converted amount.\n  hazard silent re-conversion with a stale rate distorts reported revenue.\n  favor reproducibility.',
  '["stale rate","missing timestamp","rounding asymmetry on round-trip"]',
  '["reproducibility","numerical_stability","auditability"]',
  '[]'
),
(
  'discount-code-redemption',
  'apply a discount code to an order subject to validity and usage limits',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to reduce an order total when a valid code is presented within its limits.\n  uses the @Data matching "discount code" with at-least 0.9 confidence.\n  requires the code to be active, within its date window, and below its max-use count.\n  ensures the redemption is recorded and the @Event matching "code redeemed" with at-least 0.9 confidence is emitted exactly once per order.\n  hazard concurrent redemptions exceed the usage cap.\n  favor correctness.',
  '["over-redemption race","expired code accepted","stacking beyond policy"]',
  '["correctness","auditability","determinism"]',
  '[]'
),
(
  'lexer-tokenize-pass',
  'split source text into a stream of typed tokens with span information',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to convert source text into a token stream.\n  uses the @Data matching "source buffer and token kind table" with at-least 0.9 confidence.\n  requires the source buffer to be valid utf-8.\n  ensures every byte of the source is covered by exactly one token or skipped trivia.\n  hazard ambiguous prefix between numeric literal and identifier may misclassify tokens.\n  favor determinism.',
  '["ambiguous prefix misclassification"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'parser-grammar-production',
  'reduce a token sequence into a typed syntax node according to a grammar production',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to build a syntax node from matched tokens.\n  uses the @Data matching "token cursor and production rule" with at-least 0.9 confidence.\n  requires the cursor head to match the first symbol of the production rule.\n  ensures the returned node spans a contiguous token range and the cursor advances past it.\n  hazard left-recursive productions without memoization may not terminate.\n  favor totality.',
  '["non terminating left recursion"]',
  '["totality","determinism"]',
  '[]'
),
(
  'typed-intermediate-representation',
  'represent a program as typed instructions over named values for later analysis',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to hold typed instructions and value definitions for a program unit.\n  exposes function_table as list.\n  exposes value_type_map as record.\n  exposes instruction_stream as list.\n  favor clarity.',
  '["type map drift from instructions"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'control-flow-graph-basic-block',
  'partition instructions into maximal straight-line blocks linked by control edges',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to expose basic blocks and their successor edges for one function.\n  exposes block_table as list.\n  exposes successor_edges as list.\n  exposes entry_block_id as identifier.\n  favor determinism.',
  '["unreachable block left in table"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'single-static-assignment-form',
  'rewrite a typed intermediate so each value is assigned exactly once',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to convert a typed intermediate into single-assignment form.\n  uses the @Data matching "typed ir module with control flow and dominator tree" with at-least 0.9 confidence.\n  requires the control flow graph to have a single entry block reachable from all blocks.\n  ensures every value name is the target of exactly one definition and every use is dominated by its definition.\n  hazard misplaced merge nodes at join points may shadow earlier definitions.\n  favor correctness.',
  '["misplaced merge at join"]',
  '["correctness","determinism"]',
  '[]'
),
(
  'dominator-tree-analysis',
  'compute, for each block, the set of blocks that must execute before it',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to produce the dominator tree of a control flow graph.\n  uses the @Data matching "control flow graph with reverse postorder" with at-least 0.9 confidence.\n  requires every block to be reachable from the entry block.\n  ensures the result is a tree rooted at the entry block where each node parent is its immediate dominator.\n  hazard unreachable blocks left in the input produce undefined parent pointers.\n  favor correctness.',
  '["unreachable block undefined parent"]',
  '["correctness","determinism"]',
  '[]'
),
(
  'dead-code-elimination-pass',
  'remove instructions whose results have no observable effect on program output',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to drop instructions whose values are never used and that have no side effects.\n  uses the @Data matching "typed ir module with use count and side effect tables" with at-least 0.9 confidence.\n  requires the use count table to be consistent with the current instruction stream.\n  ensures the program observable behavior on every input is unchanged.\n  hazard treating an instruction with hidden effects as pure may delete required work.\n  favor correctness.',
  '["hidden side effect treated as pure"]',
  '["correctness","determinism"]',
  '[]'
),
(
  'constant-folding-rewrite',
  'replace operations on known constant operands with their computed result',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to evaluate operations whose operands are all known at translation time.\n  uses the @Data matching "typed ir module with constant value table and operation semantics" with at-least 0.9 confidence.\n  requires the operation semantics to match the runtime semantics for every folded operation.\n  ensures the rewritten program produces the same value as the original for every input.\n  hazard folding under different rounding or overflow rules than the runtime may diverge.\n  favor numerical_stability.',
  '["rounding or overflow divergence"]',
  '["numerical_stability","correctness"]',
  '[]'
),
(
  'register-allocation-assignment',
  'assign a finite set of physical locations to the values of a function',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to map each live value to a physical register or spill slot.\n  uses the @Data matching "typed ir module with interference graph and register file" with at-least 0.9 confidence.\n  requires the interference graph to contain an edge between every pair of values live at the same point.\n  ensures no two values that interfere share the same physical register.\n  hazard missing interference edges produce silent value corruption at runtime.\n  favor correctness.',
  '["missing interference edge"]',
  '["correctness","performance"]',
  '[]'
),
(
  'source-position-span',
  'attach a half-open source byte range to a syntax or intermediate node',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to record the source byte range a node was produced from.\n  exposes source_file_id as identifier.\n  exposes start_byte_offset as integer.\n  exposes end_byte_offset as integer.\n  favor auditability.',
  '["span drifts after source rewrite"]',
  '["auditability","clarity"]',
  '[]'
);


-- Parallel-seeded batch 6 -- ML + game engine
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'training-loop-minibatch',
  'iterate a training loop over minibatches with gradient updates',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the function <name> is\n  intended to update parameters across all minibatches once.\n  uses the @Data matching "data loader with fixed-shape batches" with at-least 0.9 confidence.\n  requires the optimizer to hold the current parameter set.\n  ensures every sample contributes exactly one gradient step per epoch.\n  hazard stale gradients if the optimizer is not zeroed between steps.\n  favor reproducibility.',
  '["stale gradients if optimizer not zeroed between steps","nondeterministic batch order without seeded shuffle"]',
  '["reproducibility","determinism","numerical_stability"]',
  '[]'
),
(
  'inference-batch-scheduler',
  'coalesce incoming requests into batches for throughput-bound inference',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to group pending requests into a batch bounded by size and wait budget.\n  uses the @Data matching "request queue with max batch size" with at-least 0.9 confidence.\n  requires the wait budget to be finite.\n  ensures each accepted request either joins a batch within the wait budget or fails fast.\n  hazard head-of-line blocking if a single slow request stalls batch dispatch.\n  favor latency.',
  '["head-of-line blocking from slow requests","tail latency spike when wait budget exceeds deadline"]',
  '["latency","performance","availability"]',
  '[]'
),
(
  'embedding-vector-store',
  'store dense vectors keyed by id with approximate nearest neighbor lookup',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","exposes","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to persist embedding vectors and retrieve nearest neighbors by cosine distance.\n  uses the @Data matching "vector index with payload table" with at-least 0.9 confidence.\n  composes the @Function matching "insert-query-rebuild" with at-least 0.9 confidence.\n  requires all vectors to share a fixed dimension and unit norm.\n  ensures a k-nearest query returns k results ordered by distance with a documented recall bound.\n  exposes query and insert operations.\n  favor performance.',
  '["dimension mismatch on insert silently corrupts index","recall collapses when unit-norm invariant is violated"]',
  '["performance","correctness","reproducibility"]',
  '[]'
),
(
  'tokenizer-subword-split',
  'split raw text into subword units using a learned merge table',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to convert a string into a sequence of subword ids.\n  uses the @Data matching "merge table with vocabulary" with at-least 0.9 confidence.\n  requires the merge table to be sorted by merge rank and the vocabulary to cover all base bytes.\n  ensures every input byte maps to at least one id and decoding round-trips to the original string.\n  hazard silent id drift if the merge table version differs between train and serve.\n  favor determinism.',
  '["merge-table version drift between train and serve","round-trip failure on unnormalized input"]',
  '["determinism","reproducibility","correctness"]',
  '[]'
),
(
  'attention-score-softmax',
  'compute normalized attention weights from query-key scores',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to turn raw query-key scores into a probability distribution over keys.\n  uses the @Data matching "score tensor with mask and scale" with at-least 0.9 confidence.\n  requires the score shape to match the mask shape and the scale to be positive.\n  ensures output rows sum to one and masked positions contribute zero weight.\n  hazard overflow when scores are not shifted by row max before exponentiation.\n  favor numerical_stability.',
  '["exp overflow without max-shift","nan propagation from fully masked rows"]',
  '["numerical_stability","correctness","determinism"]',
  '[]'
),
(
  'learning-rate-warmup-schedule',
  'ramp learning rate from zero to target then decay',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return the learning rate for a given training step under a warmup-then-decay policy.\n  uses the @Data matching "step with warmup and total steps and peak rate" with at-least 0.9 confidence.\n  requires the warmup steps to be less than total steps and the peak rate to be positive.\n  ensures the returned rate is zero at step zero, equals the peak at the warmup boundary, and is non-negative thereafter.\n  hazard divergence if the peak is reached before gradient statistics have stabilized.\n  favor numerical_stability.',
  '["divergence from too-short warmup","step-counter desync across resumed runs"]',
  '["numerical_stability","reproducibility","determinism"]',
  '[]'
),
(
  'checkpoint-snapshot-resume',
  'persist and restore full training state for fault-tolerant resume',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","exposes","favor"]',
  '["@Data","@Function"]',
  'the concept <name> is\n  intended to capture parameters, optimizer state, step counter, and data-loader position so training can resume bit-identically.\n  uses the @Data matching "snapshot storage with serializer" with at-least 0.9 confidence.\n  composes the @Function matching "save-load-verify snapshot" with at-least 0.9 confidence.\n  requires the serializer to be deterministic and the storage to support atomic replace.\n  ensures a resumed run produces the same next step as an uninterrupted run from the same snapshot.\n  exposes save and load operations.\n  favor reproducibility.',
  '["partial write leaves corrupt snapshot without atomic replace","loader position lost so samples are revisited or skipped"]',
  '["reproducibility","determinism","auditability"]',
  '[]'
),
(
  'dataset-stream-shard',
  'stream a sharded dataset across workers without duplication',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to yield samples assigned to one worker from a globally sharded dataset.\n  uses the @Data matching "shard list with worker id and world size and seed" with at-least 0.9 confidence.\n  requires the worker id to be less than the world size and the shard count to be divisible by the world size or padded.\n  ensures every sample is visited by exactly one worker per epoch under a fixed seed.\n  hazard sample duplication when the world size changes mid-epoch without re-sharding.\n  favor determinism.',
  '["sample duplication on world-size change","skew when shard sizes differ and no padding is applied"]',
  '["determinism","reproducibility","performance"]',
  '[]'
),
(
  'model-quantization-reduce',
  'reduce parameter precision from float to lower-bit integer representation',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to map float parameters to lower-bit integers with per-tensor or per-channel scales.\n  uses the @Data matching "weight tensor with bit-width and policy" with at-least 0.9 confidence.\n  requires the bit-width to be in the supported set and the weights to contain no nan or inf.\n  ensures dequantized weights stay within a documented error bound of the originals.\n  hazard accuracy collapse when outlier channels are not handled by per-channel scales.\n  favor performance.',
  '["outlier-channel accuracy collapse","silent saturation when calibration range is too tight"]',
  '["performance","numerical_stability","portability"]',
  '[]'
),
(
  'feature-normalization-standardize',
  'standardize input features to zero mean and unit variance using training statistics',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to rescale features using mean and variance computed on the training split.\n  uses the @Data matching "mean and variance with epsilon" with at-least 0.9 confidence.\n  requires the mean and variance to share the shape of a feature row and the epsilon to be positive.\n  ensures output features have zero mean and unit variance on the training split up to the epsilon tolerance.\n  hazard train-serve skew if the statistics are recomputed on serving data.\n  favor statistical_rigor.',
  '["train-serve skew from recomputed stats at serving","division-by-zero when epsilon is omitted on constant features"]',
  '["statistical_rigor","numerical_stability","reproducibility"]',
  '[]'
),
(
  'game-frame-fixed-timestep',
  'advance simulation in fixed-size ticks while rendering at display rate',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to drain accumulated wall time into discrete simulation ticks.\n  uses the @Data matching "tick accumulator in seconds" with at-least 0.9 confidence.\n  uses the @Event matching "frame boundary reached" with at-least 0.9 confidence.\n  requires the fixed step size to be positive and constant across a run.\n  ensures the number of simulation steps is a deterministic function of elapsed wall time.\n  hazard unbounded catch-up on long pauses spirals into a death loop.\n  favor determinism.',
  '["death-spiral under long stalls","render interpolation drift if residual not exposed"]',
  '["determinism","reproducibility"]',
  '[]'
),
(
  'entity-component-system-query',
  'iterate entities matching a component signature in archetype-packed order',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to yield entity handles whose components satisfy a required signature.\n  uses the @Data matching "archetype storage table with signature mask" with at-least 0.9 confidence.\n  requires the signature mask to reference only registered component kinds.\n  ensures iteration visits each matching entity exactly once per pass.\n  hazard mutating the component set during iteration invalidates archetype pointers.\n  favor performance.',
  '["archetype invalidation mid-iteration","false sharing across worker lanes"]',
  '["performance","determinism"]',
  '[]'
),
(
  'rigid-body-integration-step',
  'integrate linear and angular state of a rigid body over one fixed step',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to update position, orientation, and velocities from accumulated forces and torques.\n  uses the @Data matching "rigid body state with force and torque accumulator" with at-least 0.9 confidence.\n  requires inverse mass and inverse inertia tensor to be finite and non-negative.\n  ensures the step is symplectic and clears the force accumulator before returning.\n  hazard explicit Euler with large step sizes accumulates energy and destabilizes stacks.\n  favor numerical_stability.',
  '["energy drift under explicit Euler","tunneling at high velocity"]',
  '["numerical_stability","determinism"]',
  '[]'
),
(
  'collision-broad-phase-prune',
  'reduce candidate collision pairs to those with overlapping bounding volumes',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to emit candidate pairs whose axis-aligned bounds currently overlap.\n  uses the @Data matching "bounding volume table with sweep axis endpoints" with at-least 0.9 confidence.\n  requires bounding volumes to fully enclose their underlying shape each tick.\n  ensures every truly colliding pair appears in the output set.\n  hazard loose bounds inflate the pair count and swamp the narrow phase.\n  favor performance.',
  '["loose bounds over-generate pairs","sort instability across ticks breaks determinism"]',
  '["performance","determinism"]',
  '[]'
),
(
  'spatial-hash-grid-partition',
  'bucket moving objects into a uniform grid for neighbor lookup',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to assign each object to every grid cell its bounds overlap.\n  uses the @Data matching "spatial hash cell table with object bounds" with at-least 0.9 confidence.\n  requires the chosen cell size to be larger than the typical object radius.\n  ensures a neighbor query visits only cells overlapping the query region.\n  hazard cell size far from object size either over-populates buckets or balloons occupancy lists.\n  favor performance.',
  '["hash collisions in dense regions","cell-size mismatch degrades to linear scan"]',
  '["performance","determinism"]',
  '[]'
),
(
  'input-action-binding',
  'translate raw device signals into named gameplay actions',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Event","@Data"]',
  'the function <name> is\n  intended to map raw device events to a named gameplay action with a value.\n  uses the @Event matching "raw device input event" with at-least 0.9 confidence.\n  uses the @Data matching "action binding table" with at-least 0.9 confidence.\n  requires each binding to reference an action declared in the binding table.\n  ensures an action fires at most once per input event per frame.\n  hazard unresolved rebinds during play can strand the player in an unreachable state.\n  favor accessibility.',
  '["rebind races drop inputs","chorded bindings mask single-key actions"]',
  '["accessibility","responsiveness"]',
  '[]'
),
(
  'asset-load-budget',
  'admit asset load jobs only while a per-frame time budget remains',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to dispatch pending asset loads until the frame budget is exhausted.\n  uses the @Data matching "asset load job queue with frame budget" with at-least 0.9 confidence.\n  requires the budget to be positive and less than the frame interval.\n  ensures admitted job cost estimates sum below the remaining budget for the frame.\n  hazard underestimated job cost overruns the budget and causes a visible hitch.\n  favor responsiveness.',
  '["cost-estimate drift causes hitches","starvation of low-priority assets"]',
  '["responsiveness","availability"]',
  '[]'
),
(
  'sprite-atlas-packing',
  'pack sprite rectangles into a single texture atlas with no overlap',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Media","@Data"]',
  'the function <name> is\n  intended to place each sprite rectangle into atlas coordinates without overlap.\n  uses the @Media matching "sprite source image" with at-least 0.9 confidence.\n  uses the @Data matching "atlas free rectangle list" with at-least 0.9 confidence.\n  requires every sprite rectangle to fit inside the atlas dimensions.\n  ensures the packed rectangles are pairwise non-overlapping and in-bounds.\n  hazard tight packing without padding bleeds neighboring texels under bilinear sampling.\n  favor performance.',
  '["texel bleed without padding","rotation choices break deterministic layouts"]',
  '["performance","reproducibility"]',
  '[]'
),
(
  'camera-frustum-culling',
  'discard scene objects whose bounds lie fully outside the view frustum',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to retain only objects whose bounds intersect the camera frustum.\n  uses the @Data matching "camera frustum plane set with scene object bounds" with at-least 0.9 confidence.\n  requires the six frustum planes to have inward-pointing unit normals.\n  ensures any visible object survives the cull.\n  hazard conservative plane tests retain objects slightly beyond the edge but must never drop visible ones.\n  favor correctness.',
  '["plane-normal sign flip drops visible objects","reversed-Z depth confuses near-plane test"]',
  '["correctness","performance"]',
  '[]'
),
(
  'scene-graph-world-transform',
  'compose world transforms from parent chains in a scene graph',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to derive each node world transform from its local transform and ancestors.\n  uses the @Data matching "scene graph parent chain with local transforms" with at-least 0.9 confidence.\n  requires the scene graph to contain no cycles.\n  ensures each node is updated after all its ancestors in the pass.\n  hazard stale world transforms persist when a parent reparents mid-frame without re-marking descendants.\n  favor determinism.',
  '["reparent leaves dirty descendants","non-uniform scale cascades skew normals"]',
  '["determinism","correctness"]',
  '[]'
);


-- Parallel-seeded batch 7 -- NLP + time + geospatial
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'sentence-boundary-detect',
  'segment a text stream into sentence spans using punctuation and casing cues',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to divide a text buffer into ordered sentence spans.\n  uses the @Data matching "token stream with offsets" with at-least 0.9 confidence.\n  requires the input to be decoded unicode text.\n  ensures spans cover the input without overlap and preserve byte offsets.\n  hazard abbreviations and decimal numerals may trigger false boundaries.\n  favor correctness.',
  '["abbreviation false split","decimal false split","ellipsis ambiguity"]',
  '["correctness","determinism"]',
  '[]'
),
(
  'stopword-filter-pass',
  'drop high-frequency function words from a token sequence before downstream scoring',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to remove high-frequency function words from a token list.\n  uses the @Data matching "curated stopword set for the target register" with at-least 0.9 confidence.\n  requires tokens to be lowercase-normalized.\n  ensures original token order is preserved for surviving tokens.\n  hazard aggressive lists erase negation and quantifier cues that flip downstream meaning.\n  favor clarity.',
  '["loses negation","register mismatch","erases quantifiers"]',
  '["clarity","performance"]',
  '[]'
),
(
  'stemming-lemma-normalize',
  'collapse inflected surface forms to a shared root for bag-of-token matching',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to map each surface token to a canonical root form.\n  uses the @Data matching "morphology table for the source locale" with at-least 0.9 confidence.\n  requires the token locale to be known before lookup.\n  ensures tokens sharing a lemma collapse to the same output string.\n  hazard overstemming merges unrelated senses and yields downstream precision loss.\n  favor determinism.',
  '["overstemming","understemming","locale drift"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'named-entity-span-tag',
  'label contiguous token spans with entity categories such as person, place, or organization',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to assign entity category labels to contiguous token spans.\n  uses the @Data matching "tokenized sentence with part-of-speech tags" with at-least 0.9 confidence.\n  requires sentence boundaries to be resolved upstream.\n  ensures spans do not overlap and each label comes from the declared category set.\n  hazard novel or mixed-vocabulary entities silently receive the generic fallback label.\n  favor auditability.',
  '["out-of-vocabulary entity","mixed-vocabulary drop","overlapping spans"]',
  '["auditability","correctness"]',
  '[]'
),
(
  'part-of-speech-tag',
  'assign a grammatical category to every token in a sentence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to emit a grammatical category for each input token.\n  uses the @Data matching "token sequence with sentence context" with at-least 0.9 confidence.\n  requires the tag inventory to be fixed before inference.\n  ensures output length equals input token count and every tag is drawn from the fixed inventory.\n  hazard rare words and zero-context fragments fall back to the most frequent class.\n  favor determinism.',
  '["rare-word fallback","zero-context fragment","tag inventory drift"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'text-similarity-score',
  'return a bounded similarity score between two text passages using vector overlap',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to measure semantic closeness between two text passages.\n  uses the @Data matching "shared token or embedding space for both inputs" with at-least 0.9 confidence.\n  requires both inputs to share the same normalization pipeline.\n  ensures the returned score lies in the closed interval zero to one and is symmetric.\n  hazard length asymmetry inflates scores when one passage is a strict substring of the other.\n  favor numerical_stability.',
  '["length asymmetry","normalization mismatch","vocabulary gap"]',
  '["numerical_stability","reproducibility"]',
  '[]'
),
(
  'language-detection-guess',
  'predict the language of a short text passage from character and n-gram signals',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return the most probable language label for a text passage.\n  uses the @Data matching "character n-gram profile per candidate language" with at-least 0.9 confidence.\n  requires at least one non-whitespace grapheme in the input.\n  ensures the returned label belongs to the declared candidate set and carries a confidence score.\n  hazard short inputs and mixed-language passages produce unstable guesses.\n  favor reproducibility.',
  '["short-input instability","mixed-language passage","script collision"]',
  '["reproducibility","determinism"]',
  '[]'
),
(
  'spell-correction-edit-distance',
  'suggest the closest dictionary word for an unknown token using bounded edit distance',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to propose the nearest dictionary word for an unknown token.\n  uses the @Data matching "ranked dictionary keyed by frequency" with at-least 0.9 confidence.\n  requires a maximum edit distance bound to be supplied by the caller.\n  ensures every candidate returned lies within the supplied distance bound.\n  hazard proper nouns and domain jargon get rewritten toward common dictionary words.\n  favor clarity.',
  '["proper-noun rewrite","jargon collapse","frequency bias"]',
  '["clarity","correctness"]',
  '[]'
),
(
  'topic-keyword-extract',
  'extract a ranked list of salient keywords representing the topic of a document',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return a ranked list of keywords that summarize a document.\n  uses the @Data matching "document with computed term frequency statistics" with at-least 0.9 confidence.\n  requires the document to be tokenized and stopword-filtered upstream.\n  ensures the output list is bounded in length and ordered by descending salience.\n  hazard boilerplate headers and repeated navigation text dominate the ranking.\n  favor clarity.',
  '["boilerplate dominance","short-document noise","repeated-phrase bias"]',
  '["clarity","discoverability"]',
  '[]'
),
(
  'sentiment-polarity-score',
  'score a passage on a bounded polarity axis from negative through neutral to positive',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to place a passage on a bounded sentiment polarity axis.\n  uses the @Data matching "polarity lexicon with negation and intensifier cues" with at-least 0.9 confidence.\n  requires sentence boundaries and negation scope to be resolved upstream.\n  ensures the returned score lies in the closed interval negative one to positive one.\n  hazard sarcasm and figurative language invert the surface polarity.\n  favor auditability.',
  '["sarcasm inversion","figurative language","domain-shift lexicon"]',
  '["auditability","reproducibility"]',
  '[]'
),
(
  'monotonic-clock-reading',
  'read a monotonic clock source that never moves backward',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return a tick count from a source that never decreases across calls within one process.\n  uses the @Data matching "monotonic tick source exposing a strictly non-decreasing counter" with at-least 0.9 confidence.\n  requires the caller to hold no assumption that ticks map to wall-clock seconds.\n  ensures two successive calls on the same thread return values where the later value is at least the earlier value.\n  hazard tick counters wrap on long-running processes and must be widened before subtraction.\n  favor determinism.',
  '["counter wrap","unit confusion"]',
  '["determinism","correctness","reproducibility"]',
  '[]'
),
(
  'wall-clock-timestamp',
  'capture a wall-clock instant with explicit timezone and epoch',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to record a wall-clock instant as an epoch offset plus a named timezone.\n  uses the @Data matching "instant record carrying epoch seconds, sub-second fraction, and zone identifier" with at-least 0.9 confidence.\n  requires the host clock to be synchronized within a declared skew budget before capture.\n  ensures the returned record round-trips through serialization without losing zone or sub-second fraction.\n  hazard wall clocks can jump backward during synchronization and must never be used to measure elapsed duration.\n  favor auditability.',
  '["clock jump","zone loss","leap smear"]',
  '["auditability","correctness","portability"]',
  '[]'
),
(
  'duration-arithmetic',
  'add and subtract durations with checked overflow and consistent units',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to combine two signed durations expressed in the same base unit and return a single duration or an overflow marker.\n  uses the @Data matching "signed duration carrying a 64-bit count and a fixed base unit tag" with at-least 0.9 confidence.\n  requires both inputs to share the same base unit tag before addition proceeds.\n  ensures any result that would exceed the signed 64-bit range is reported as an overflow marker rather than wrapping silently.\n  hazard mixing base units without conversion yields numerically plausible but semantically wrong results.\n  favor numerical_stability.',
  '["unit mismatch","silent overflow"]',
  '["numerical_stability","correctness","totality"]',
  '[]'
),
(
  'calendar-date-component',
  'decompose an instant into calendar year month day fields under a stated zone',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to project a wall instant into year month day hour minute second fields under a named timezone.\n  uses the @Data matching "calendar field record with year, month, day, hour, minute, second, and zone identifier" with at-least 0.9 confidence.\n  requires the timezone rules database consulted during projection to be versioned and pinned for the caller.\n  ensures projecting an instant and then recomposing it under the same zone version yields the original instant.\n  hazard daylight-saving transitions produce ambiguous or non-existent local times that must be resolved by an explicit policy.\n  favor reproducibility.',
  '["dst ambiguity","zone db drift"]',
  '["reproducibility","correctness","auditability"]',
  '[]'
),
(
  'recurrence-rule-expander',
  'expand a bounded recurrence rule into a finite list of occurrence instants',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to produce every occurrence instant of a recurrence rule that falls within a closed start and end window.\n  uses the @Data matching "recurrence specification with frequency, interval, weekday mask, month mask, and optional count limit" with at-least 0.9 confidence.\n  requires the window end to be finite and the rule to declare either a count cap or an until instant.\n  ensures the returned list is sorted ascending, contains no duplicates, and every element lies inside the requested window.\n  hazard unbounded rules without a count cap or until instant can produce effectively infinite expansion and must be rejected.\n  favor totality.',
  '["unbounded expansion","zone drift"]',
  '["totality","determinism","correctness"]',
  '[]'
),
(
  'business-day-calendar',
  'test whether a calendar date counts as a working day under a named calendar',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to decide whether a given calendar date counts as a working day under a named working-calendar.\n  uses the @Data matching "working-calendar record holding weekend mask, holiday dates, and jurisdiction tag" with at-least 0.9 confidence.\n  requires the working-calendar to be loaded from a versioned source pinned by the caller.\n  ensures the decision depends only on the date fields and the pinned calendar version, never on the current wall clock.\n  hazard holiday lists diverge across jurisdictions and must never be silently substituted for one another.\n  favor auditability.',
  '["jurisdiction swap","stale holiday list"]',
  '["auditability","reproducibility","correctness"]',
  '[]'
),
(
  'timer-delayed-fire',
  'schedule a callback to fire after a minimum delay against a monotonic source',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to arrange for a callback to be invoked no earlier than a given delay measured on a monotonic source.\n  uses the @Data matching "timer request holding a callback handle, a minimum delay, and a monotonic source reference" with at-least 0.9 confidence.\n  requires the delay to be non-negative and expressed in the same unit as the chosen monotonic source.\n  ensures the callback, when invoked, observes a monotonic reading at or after the scheduled fire tick.\n  hazard wall-clock jumps during sleep can distort delay if the scheduler falls back to wall time.\n  favor latency.',
  '["wall fallback","cancellation race"]',
  '["latency","determinism","responsiveness"]',
  '[]'
),
(
  'clock-skew-detection',
  'compare two clock sources and flag drift beyond a declared budget',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to compare a local clock reading to a reference clock reading and flag drift beyond a declared budget.\n  uses the @Data matching "skew report holding local instant, reference instant, measured offset, and budget" with at-least 0.9 confidence.\n  requires both readings to be captured within a network round-trip bound recorded on the report.\n  ensures the report classifies the local clock as within-budget, lagging, or leading relative to the reference.\n  hazard asymmetric network paths bias the measured offset and must be bounded by the recorded round-trip.\n  favor auditability.',
  '["path asymmetry","round-trip inflation"]',
  '["auditability","correctness","availability"]',
  '[]'
),
(
  'leap-second-handling',
  'resolve leap-second insertions under an explicit smoothing policy',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to map an instant that falls inside a leap-second window onto a linear timeline under a stated smoothing policy.\n  uses the @Data matching "leap policy record naming one of strict-insertion, linear-smear, or step-and-hold" with at-least 0.9 confidence.\n  requires the caller to declare the smoothing policy before any instant is resolved under it.\n  ensures two instants resolved under the same policy preserve their strict ordering after resolution.\n  hazard switching policies mid-stream breaks ordering guarantees and must be refused.\n  favor determinism.',
  '["policy switch","ordering break"]',
  '["determinism","correctness","reproducibility"]',
  '[]'
),
(
  'interval-overlap-test',
  'decide whether two half-open time intervals overlap under a single timeline',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to decide whether two half-open time intervals share at least one instant on a single timeline.\n  uses the @Data matching "half-open interval pair with start-inclusive and end-exclusive instants on a shared timeline" with at-least 0.9 confidence.\n  requires both intervals to carry the same timeline tag and each start to be at most its own end.\n  ensures the decision is symmetric in its two inputs and returns false when either interval is empty.\n  hazard comparing intervals from different timelines without conversion produces meaningless overlaps.\n  favor correctness.',
  '["timeline mismatch","empty interval"]',
  '["correctness","totality","clarity"]',
  '[]'
),
(
  'geographic-coordinate-point',
  'represent a latitude-longitude point with datum awareness',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to hold a surface location as latitude and longitude with a stated datum.\n  exposes latitude as real.\n  exposes longitude as real.\n  exposes datum as identifier.\n  favor portability.',
  '["silent datum mismatch","out-of-range latitude","axis-order confusion"]',
  '["portability","correctness","clarity"]',
  '[]'
),
(
  'coordinate-reference-projection',
  'project coordinates between a source and target reference system',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to map a point from a source reference system into a target reference system.\n  uses the @Data matching "source and target reference identifiers" with at-least 0.9 confidence.\n  requires both reference systems to be registered and resolvable.\n  ensures the returned point carries the target reference identifier.\n  hazard silent loss of precision near projection boundaries.\n  favor numerical_stability.',
  '["precision loss at projection edges","unregistered reference identifier","wrong axis order after projection"]',
  '["numerical_stability","correctness","portability"]',
  '[]'
),
(
  'spatial-index-quadtree',
  'index two-dimensional points for fast region queries',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","exposes","favor"]',
  '["@Data"]',
  'the concept <name> is\n  intended to partition a bounded plane recursively so that region queries return only nearby points.\n  uses the @Data matching "bounding rectangle" with at-least 0.9 confidence.\n  composes a tree of four child quadrants per node.\n  requires every inserted point to lie inside the root bounding rectangle.\n  ensures a region query returns every indexed point that intersects the query rectangle.\n  exposes insert and query-region operations.\n  favor performance.',
  '["unbounded recursion on coincident points","memory blowup at high density","stale index after mutation"]',
  '["performance","correctness","determinism"]',
  '[]'
),
(
  'geofence-membership-check',
  'decide whether a point lies inside a named polygonal fence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to report whether a surface point lies inside a closed fence polygon.\n  uses the @Data matching "fence polygon and query point" with at-least 0.9 confidence.\n  requires the fence polygon to be simple and closed.\n  ensures the result is a decision of inside, outside, or on-boundary.\n  hazard ambiguous verdict for points exactly on an edge.\n  favor determinism.',
  '["boundary point ambiguity","self-intersecting polygon","antimeridian wrap not handled"]',
  '["determinism","correctness","clarity"]',
  '[]'
),
(
  'route-shortest-path',
  'find a lowest-cost route between two nodes in a weighted graph',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return the lowest-cost sequence of edges from a start node to a goal node.\n  uses the @Data matching "weighted road graph with start and goal identifiers" with at-least 0.9 confidence.\n  requires every edge weight to be non-negative and finite.\n  ensures no cheaper path exists in the graph between the same endpoints.\n  hazard infinite loop on negative-weight cycle.\n  favor minimum_cost.',
  '["negative edge weight","disconnected endpoints","exploding frontier on dense graph"]',
  '["minimum_cost","correctness","performance"]',
  '[]'
),
(
  'address-geocoding-lookup',
  'resolve a postal address string to a coordinate with a confidence score',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to turn a free-form postal address into a coordinate point with a confidence score.\n  uses the @Data matching "raw address text and address gazetteer" with at-least 0.9 confidence.\n  requires the gazetteer to be loaded and non-empty.\n  ensures every returned coordinate carries a score between zero and one.\n  hazard ambiguous address resolving to an unrelated locality.\n  favor auditability.',
  '["ambiguous address match","stale gazetteer","silent fallback to centroid"]',
  '["auditability","correctness","clarity"]',
  '[]'
),
(
  'tile-pyramid-addressing',
  'address map tiles by zoom-level and integer coordinates',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","exposes","favor"]',
  '["@Data"]',
  'the concept <name> is\n  intended to address map tiles by a zoom level and a pair of integer tile coordinates.\n  uses the @Data matching "zoom level" with at-least 0.9 confidence.\n  composes a pyramid where each zoom step quadruples the tile count.\n  requires tile coordinates to fit inside the bounds implied by the zoom level.\n  ensures every surface location maps to exactly one tile at each zoom.\n  exposes tile-for-point and bounds-for-tile operations.\n  favor reproducibility.',
  '["off-by-one tile at boundary","axis flip between tile addressings","unbounded zoom requests"]',
  '["reproducibility","determinism","portability"]',
  '[]'
),
(
  'distance-on-sphere',
  'measure great-circle distance between two surface points',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return the great-circle distance in meters between two surface points on a reference sphere.\n  uses the @Data matching "two surface points with shared datum" with at-least 0.9 confidence.\n  requires both points to be finite latitude-longitude pairs in the same datum.\n  ensures the returned distance is non-negative and symmetric in its inputs.\n  hazard catastrophic cancellation for near-antipodal point pairs.\n  favor numerical_stability.',
  '["near-antipodal cancellation","datum mismatch between inputs","confusing spheroid with sphere"]',
  '["numerical_stability","correctness","determinism"]',
  '[]'
),
(
  'polygon-clip-intersection',
  'clip one polygon against another and return the intersection region',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return the polygon region that is inside both input polygons.\n  uses the @Data matching "subject polygon and clip polygon" with at-least 0.9 confidence.\n  requires both polygons to be simple and closed.\n  ensures every returned ring is simple, closed, and contained in both inputs.\n  hazard degenerate slivers at near-collinear edges.\n  favor numerical_stability.',
  '["degenerate sliver output","self-intersecting input","precision loss near collinear edges"]',
  '["numerical_stability","correctness","determinism"]',
  '[]'
),
(
  'time-zone-aware-timestamp',
  'stamp a surface event with an instant and its local time zone',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to record the instant and local time zone at which a surface event occurred.\n  exposes instant as real.\n  exposes zone as identifier.\n  exposes offset_minutes as real.\n  favor auditability.',
  '["ambiguous instant during daylight transition","stale zone identifier","offset drift over historical dates"]',
  '["auditability","correctness","reproducibility"]',
  '[]'
);


-- Parallel-seeded batch 8 -- IoT + bioinformatics + robotics
INSERT OR IGNORE INTO patterns (
  pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs,
  example_shape, hazards, favors, source_doc_refs
) VALUES
(
  'sensor-sample-debounce',
  'periodically sample a noisy input line and emit a stable reading only after the signal has held its new state for a minimum dwell window',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to produce a stable boolean reading from a bouncing input line.\n  uses the @Data matching "rolling sample window with monotonic timestamp" with at-least 0.9 confidence.\n  requires dwell window to be shorter than the fastest legitimate edge interval.\n  ensures no transition is reported until the raw reading has held for at least the dwell window.\n  hazard spurious transitions when the dwell window is tuned below physical bounce duration.\n  favor determinism.',
  '["spurious transition","sampling aliasing"]',
  '["determinism","numerical_stability"]',
  '[]'
),
(
  'interrupt-service-routine',
  'respond to a hardware event with a minimal routine that acknowledges the source and defers heavier work',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Event","@Data"]',
  'the function <name> is\n  intended to acknowledge a hardware edge and hand off a timestamped record to a deferred worker.\n  uses the @Event matching "edge-triggered line assertion" with at-least 0.95 confidence.\n  requires the handler to avoid blocking calls, heap allocation, and unbounded loops.\n  ensures the interrupt source is cleared before return and exactly one record is enqueued per edge.\n  hazard priority inversion when the handler shares a lock with a lower-priority task.\n  favor latency.',
  '["priority inversion","missed edge"]',
  '["latency","determinism"]',
  '[]'
),
(
  'low-power-sleep-mode',
  'enter the deepest power state consistent with the next scheduled obligation',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to minimize average current by selecting the deepest sleep state whose wake sources still honor the next deadline.\n  uses the @Data matching "scheduled wake obligation with earliest deadline" with at-least 0.9 confidence.\n  requires all pending writes to be flushed and all wake sources to be armed before entry.\n  ensures the device resumes no later than the earliest pending deadline.\n  hazard missed wake when a required source is gated by the chosen sleep state.\n  favor minimum_cost.',
  '["missed wake","stale state on resume"]',
  '["minimum_cost","determinism"]',
  '[]'
),
(
  'over-the-air-firmware-update',
  'receive, verify, stage, atomically swap, and roll back firmware images under network delivery',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Data","@Event"]',
  'the concept <name> is\n  intended to replace the running image with a verified candidate and roll back automatically if the new image fails to confirm health.\n  uses the @Data matching "signed image manifest with version and digest" with at-least 0.95 confidence.\n  composes the @Function matching "download-verify-stage-swap-health_confirm" with at-least 0.9 confidence.\n  requires a backup slot large enough for the full image and a persistent boot-selector.\n  ensures the device never boots an unverified image and reverts to the prior image if health is not confirmed within a bounded window.\n  hazard bricked device when the boot-selector is written non-atomically.\n  favor auditability.',
  '["bricked device","unverified image","rollback loop"]',
  '["auditability","correctness","availability"]',
  '[]'
),
(
  'watchdog-timer-reset',
  'periodically prove liveness to an independent timer so a stuck path forces a clean reset',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to refresh an independent liveness timer from the main supervisory loop at a cadence shorter than the timer period.\n  uses the @Data matching "supervisory loop heartbeat counter" with at-least 0.9 confidence.\n  requires the refresh site to be reachable only when all critical subsystems have reported progress this cycle.\n  ensures a stalled or diverged firmware path triggers a reset within one timer period.\n  hazard masked faults when the refresh is moved into an interrupt that keeps firing despite the main loop being stuck.\n  favor availability.',
  '["masked fault","reset loop"]',
  '["availability","determinism"]',
  '[]'
),
(
  'telemetry-batch-upload',
  'accumulate measurement records locally and upload them in bounded batches on a duty cycle',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data","@Event"]',
  'the function <name> is\n  intended to upload accumulated records in one link wake whenever the batch is full or the oldest record has aged past the staleness bound.\n  uses the @Data matching "append-only local record buffer with per-record timestamp" with at-least 0.9 confidence.\n  requires the buffer to be persisted across resets and the upload to be idempotent under retry.\n  ensures every record is delivered at most once and no later than the staleness bound.\n  hazard unbounded buffer growth when the link is unavailable longer than the staleness bound allows.\n  favor minimum_cost.',
  '["buffer overflow","duplicate delivery"]',
  '["minimum_cost","reproducibility"]',
  '[]'
),
(
  'device-twin-state-sync',
  'reconcile local authoritative state with a mirrored remote representation under intermittent connectivity',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Data","@Function","@Event"]',
  'the concept <name> is\n  intended to keep the device observed state and its remote mirror convergent under intermittent connectivity.\n  uses the @Data matching "per-field version vector with monotonic clock" with at-least 0.9 confidence.\n  composes the @Function matching "local-apply and remote-publish and remote-accept" with at-least 0.9 confidence.\n  requires every field update to carry a monotonic version and an originator identity.\n  ensures after a quiescent interval the device and mirror agree field-by-field on the highest-version value.\n  hazard clock skew producing field-level regression when two originators race.\n  favor correctness.',
  '["clock skew","write loss","convergence stall"]',
  '["correctness","auditability"]',
  '[]'
),
(
  'ring-buffer-bounded-producer',
  'hand records from a fast producer to a slower consumer through a fixed-capacity circular buffer',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Function"]',
  'the data <name> is\n  intended to carry records from one producer to one consumer in first-in first-out order within a fixed memory footprint.\n  exposes push and pop and length and is-full operations with constant time and no heap allocation.\n  favor determinism.',
  '["drop on full","overwrite on full","torn read"]',
  '["determinism","latency","performance"]',
  '[]'
),
(
  'critical-section-lock-irq',
  'protect a short shared-state update against a specific interrupt by masking it for the minimum span',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to execute a short shared-state update atomically with respect to a specific interrupt source.\n  uses the @Data matching "prior interrupt mask state saved on entry" with at-least 0.95 confidence.\n  requires the guarded span to contain no blocking call, no allocation, and no nested mask of a lower-priority source.\n  ensures the mask is restored to its prior state on every exit path including panic and early return.\n  hazard lost interrupts when the masked span exceeds the source minimum reassertion interval.\n  favor determinism.',
  '["lost interrupt","priority inversion","deadlock"]',
  '["determinism","latency"]',
  '[]'
),
(
  'real-time-deadline-scheduler',
  'dispatch a declared set of periodic tasks so that each completes before its deadline',
  '["concept"]',
  '["intended","uses","composes","requires","ensures","hazard","favor"]',
  '["@Function","@Data"]',
  'the concept <name> is\n  intended to run a declared set of periodic tasks so each task finishes before its deadline on every release.\n  uses the @Data matching "task set with period and worst-case execution time and deadline per task" with at-least 0.95 confidence.\n  composes the @Function matching "admission-release-preempt" with at-least 0.9 confidence.\n  requires each task worst-case execution time to be measured not estimated and the aggregate utilization to pass the admission test.\n  ensures no admitted task misses its deadline under the declared task set.\n  hazard deadline miss when a task actual execution time exceeds its declared worst case.\n  favor determinism.',
  '["deadline miss","starvation","priority inversion"]',
  '["determinism","latency","correctness"]',
  '[]'
),
(
  'sequence-read-quality-trim',
  'trim low-quality bases from sequencing reads using a sliding-window quality threshold',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to remove low-quality tail bases from a sequencing read.\n  uses the @Data matching "quality score array" with at-least 0.9 confidence.\n  requires every score to be a non-negative integer and the window size to divide evenly.\n  ensures the trimmed read preserves five-prime ordering and reports the cut index.\n  hazard over-trimming destroys variant-supporting bases near adapter boundaries.\n  favor statistical_rigor.',
  '["over trim","adapter contamination"]',
  '["statistical_rigor","correctness"]',
  '[]'
),
(
  'reference-alignment-score',
  'score a read-to-reference alignment using match, mismatch, and affine-gap penalties',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to compute an affine-gap alignment score between a read and a reference window.\n  uses the @Data matching "scoring matrix" with at-least 0.9 confidence.\n  requires the gap-open penalty to be strictly greater than the gap-extend penalty.\n  ensures the returned score is deterministic for equal inputs and bounded by read length times max-match.\n  hazard integer overflow on very long reads when using narrow accumulators.\n  favor numerical_stability.',
  '["overflow","asymmetric penalty"]',
  '["numerical_stability","determinism"]',
  '[]'
),
(
  'variant-call-evidence-filter',
  'filter candidate variants by depth, allele frequency, and strand-bias evidence',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to retain variant calls whose evidence exceeds configured depth and balance thresholds.\n  uses the @Data matching "pileup column with forward and reverse counts" with at-least 0.9 confidence.\n  requires total depth to equal the sum of per-strand observations.\n  ensures filtered variants carry a recorded rejection reason or a pass flag.\n  hazard strand-bias thresholds tuned to one chemistry silently drop real low-frequency calls on another.\n  favor statistical_rigor.',
  '["chemistry bias","threshold brittleness"]',
  '["statistical_rigor","auditability"]',
  '[]'
),
(
  'gene-expression-count-normalize',
  'normalize raw expression counts to length-and-depth-scaled values for cross-sample comparison',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to scale raw per-feature counts by effective length and library size.\n  uses the @Data matching "count matrix with rows as features and columns as samples" with at-least 0.9 confidence.\n  requires every effective length to be strictly positive.\n  ensures column sums after normalization match the configured target depth within rounding.\n  hazard zero-count features inflate variance when pseudocounts are omitted.\n  favor statistical_rigor.',
  '["zero inflation","length zero"]',
  '["statistical_rigor","reproducibility"]',
  '[]'
),
(
  'kmer-hash-index',
  'build a hashed k-mer index over a reference for constant-time seed lookup',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to map each canonical k-mer to the list of reference positions where it occurs.\n  exposes a lookup taking a k-mer and returning a position slice.\n  favor performance.',
  '["hash collision","memory blowup"]',
  '["performance","discoverability"]',
  '[]'
),
(
  'pairwise-alignment-traceback',
  'reconstruct the optimal alignment path from a filled score matrix',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to walk a filled score matrix from the highest-scoring cell back to an origin cell.\n  uses the @Data matching "score and direction matrices" with at-least 0.9 confidence.\n  requires the direction matrix to record exactly one predecessor per cell.\n  ensures the returned path corresponds to the reported score under the same penalty table.\n  hazard tie-breaking between equal-scoring predecessors yields non-deterministic paths across runs.\n  favor determinism.',
  '["tie nondeterminism","off by one"]',
  '["determinism","correctness"]',
  '[]'
),
(
  'phylogenetic-tree-distance',
  'compute a patristic distance between two taxa on a rooted branch-length tree',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to sum branch lengths along the path between two leaves through their lowest common ancestor.\n  uses the @Data matching "rooted tree with branch lengths" with at-least 0.9 confidence.\n  requires every branch length to be non-negative and finite.\n  ensures the distance is symmetric and zero when both taxa are the same leaf.\n  hazard negative or missing branch lengths silently produce misleading distances.\n  favor correctness.',
  '["missing branch","non ultrametric"]',
  '["correctness","numerical_stability"]',
  '[]'
),
(
  'motif-pattern-search',
  'search a sequence for occurrences of a degenerate motif expressed as position weights',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to report all positions where a sequence matches a position-weight motif above a score cutoff.\n  uses the @Data matching "position weight matrix" with at-least 0.9 confidence.\n  requires the motif columns to sum to a finite positive value per position.\n  ensures reported positions include both strand and score for each hit.\n  hazard low-complexity regions produce dense spurious hits that flood downstream analysis.\n  favor statistical_rigor.',
  '["low-complexity flood","cutoff sensitivity"]',
  '["statistical_rigor","clarity"]',
  '[]'
),
(
  'biochemical-reaction-stoichiometry',
  'balance a biochemical reaction by checking element and charge conservation',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to verify that atom counts and net charge match on both sides of a reaction.\n  uses the @Data matching "reaction with stoichiometric coefficients" with at-least 0.9 confidence.\n  requires every coefficient to be a positive rational number.\n  ensures a balanced reaction reports zero residual per element and a residual charge of zero.\n  hazard implicit protons or water molecules absent from the record cause false imbalance reports.\n  favor correctness.',
  '["implicit proton","implicit water"]',
  '["correctness","auditability"]',
  '[]'
),
(
  'molecular-structure-bond-record',
  'represent a small-molecule structure as atoms with coordinates and typed covalent bonds',
  '["data"]',
  '["intended","exposes","favor"]',
  '["@Data"]',
  'the data <name> is\n  intended to store atoms with three-dimensional coordinates and the typed bonds that connect them.\n  exposes queries for atom neighborhoods and bond-order lookups.\n  favor clarity.',
  '["bond order ambiguity","stereochemistry loss"]',
  '["clarity","reproducibility"]',
  '[]'
),
(
  'pid-controller-closed-loop',
  'compute actuator command from setpoint error using proportional integral derivative terms',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to drive measured state toward setpoint via weighted error terms.\n  uses the @Data matching "controller gains" with at-least 0.9 confidence.\n  requires the sample interval to be positive and the gains to be finite.\n  ensures output saturates within actuator limits and the integral term is anti-windup clamped.\n  hazard integral windup amplifies overshoot when the actuator saturates.\n  favor numerical_stability.',
  '["integral windup","derivative noise amplification"]',
  '["numerical_stability","determinism","latency"]',
  '[]'
),
(
  'motion-planner-collision-free',
  'produce a waypoint sequence from start to goal avoiding all known obstacles',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to return an ordered waypoint sequence reaching the goal without intersecting obstacles.\n  uses the @Data matching "occupancy map" with at-least 0.9 confidence.\n  requires start and goal to lie in free space.\n  ensures consecutive waypoints maintain clearance above the declared safety margin.\n  hazard narrow passages cause the planner to time out without reporting infeasibility.\n  favor correctness.',
  '["timeout without infeasibility signal","stale map"]',
  '["correctness","totality","determinism"]',
  '[]'
),
(
  'inverse-kinematics-solver',
  'solve joint configuration that places the end effector at a target pose',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to find joint angles placing the end effector within tolerance of a target pose.\n  uses the @Data matching "kinematic chain" with at-least 0.9 confidence.\n  requires the target pose to lie within the reachable workspace.\n  ensures the returned configuration respects joint limits and reports singularity proximity.\n  hazard gimbal-aligned configurations produce an ill-conditioned jacobian.\n  favor numerical_stability.',
  '["singularity","multi-solution ambiguity"]',
  '["numerical_stability","correctness","determinism"]',
  '[]'
),
(
  'sensor-fusion-kalman-filter',
  'fuse noisy sensor streams into a minimum-variance state estimate',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to combine prediction and measurement into a posterior state and covariance.\n  uses the @Data matching "process and measurement noise" with at-least 0.9 confidence.\n  requires the noise covariances to be positive semidefinite.\n  ensures the posterior covariance remains symmetric and positive semidefinite after update.\n  hazard numerical drift breaks covariance symmetry over long horizons.\n  favor numerical_stability.',
  '["covariance drift","mismodeled noise"]',
  '["numerical_stability","statistical_rigor","reproducibility"]',
  '[]'
),
(
  'trajectory-spline-interpolate',
  'interpolate a smooth time-parameterized trajectory through given waypoints',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to produce continuous position velocity and acceleration through waypoints.\n  uses the @Data matching "waypoint sequence" with at-least 0.9 confidence.\n  requires waypoint timestamps to be strictly increasing.\n  ensures the resulting curve is twice-differentiable and respects velocity bounds.\n  hazard high-degree splines oscillate between closely spaced waypoints.\n  favor numerical_stability.',
  '["spline oscillation","velocity bound violation"]',
  '["numerical_stability","clarity","determinism"]',
  '[]'
),
(
  'obstacle-avoidance-potential-field',
  'compute a steering vector from attractive goal and repulsive obstacle fields',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to sum attractive gradient toward the goal and repulsive gradients from obstacles.\n  uses the @Data matching "obstacle positions" with at-least 0.9 confidence.\n  requires the repulsive radius to be smaller than the inter-obstacle spacing.\n  ensures the returned vector magnitude is bounded by the maximum steer.\n  hazard symmetric obstacle arrangements produce local minima trapping the agent.\n  favor responsiveness.',
  '["local minimum trap","oscillation near obstacle"]',
  '["responsiveness","latency","determinism"]',
  '[]'
),
(
  'localization-particle-filter',
  'estimate a pose distribution from a motion model and observation likelihood',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to propagate particles by the motion model and reweight by observation likelihood.\n  uses the @Data matching "particle set" with at-least 0.9 confidence.\n  requires the particle count to be above the minimum effective sample size.\n  ensures resampling triggers when the effective sample size falls below the threshold.\n  hazard particle deprivation collapses belief onto the wrong mode.\n  favor statistical_rigor.',
  '["particle deprivation","sample impoverishment"]',
  '["statistical_rigor","reproducibility","correctness"]',
  '[]'
),
(
  'actuator-torque-limit',
  'clamp commanded torque to the actuator safe operating envelope',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to reduce commanded torque to within the rated envelope and report saturation.\n  uses the @Data matching "actuator limits" with at-least 0.9 confidence.\n  requires the limit envelope parameters to be non-negative and finite.\n  ensures the returned torque magnitude does not exceed the rated maximum and the saturation flag is set accurately.\n  hazard silent clamping hides control-law divergence from upstream observers.\n  favor auditability.',
  '["silent saturation","thermal envelope ignored"]',
  '["auditability","correctness","determinism"]',
  '[]'
),
(
  'emergency-stop-latch',
  'latch a safe-state output when the stop signal asserts and hold until explicitly cleared',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to transition to the safe state on stop assertion and hold regardless of further inputs.\n  uses the @Data matching "latch state" with at-least 0.9 confidence.\n  requires the clear signal to originate from an authorized reset path only.\n  ensures once latched the output remains safe until an explicit cleared transition is observed.\n  hazard a glitch on the clear line may prematurely release the latch.\n  favor determinism.',
  '["glitch release","missed assertion"]',
  '["determinism","auditability","correctness"]',
  '[]'
),
(
  'closed-loop-feedback-setpoint',
  'drive a measured output to a reference setpoint via a feedback correction law',
  '["function"]',
  '["intended","uses","requires","ensures","hazard","favor"]',
  '["@Data"]',
  'the function <name> is\n  intended to compute a correction from the error between setpoint and measurement each cycle.\n  uses the @Data matching "loop configuration" with at-least 0.9 confidence.\n  requires the loop period to be shorter than the plant dominant time constant.\n  ensures the steady-state error converges below tolerance for a step reference.\n  hazard sensor delay exceeding the loop period induces limit-cycle oscillation.\n  favor latency.',
  '["sensor delay oscillation","reference jump saturation"]',
  '["latency","numerical_stability","determinism"]',
  '[]'
);

-- ── Schema version stamp ────────────────────────────────────────────

INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('baseline_version', '1.0');

COMMIT;
