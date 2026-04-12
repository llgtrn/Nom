//! Parser subset probe — documents which aspirational features the
//! current nom-parser accepts by running a suite of small source
//! snippets.
//!
//! Observation from the first run (2026-04-13): `parse_source()` is
//! lenient — it recovers from unrecognized syntax and returns Ok with
//! warnings printed to stderr, rather than returning Err. Tuple
//! returns and `list[T]` both "parse" in this weak sense. That means
//! the rejection tests below aren't about strict parser-rejection;
//! they're about **whether the parse produces a usable declaration**.
//!
//! Drift detection:
//!   - an `expect_accepts` test fails → accepted feature regressed.
//!   - an `expect_parses_but_loses_shape` test fails → the parser
//!     now retains the shape (generic/tuple/etc.) instead of
//!     recovering with a warning. Move the feature to `expect_accepts`
//!     + update stdlib/self_host/README.md + extend scaffolds.

fn try_parse(src: &str) -> Result<(), String> {
    nom_parser::parse_source(src)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn expect_accepts(src: &str, label: &str) {
    if let Err(e) = try_parse(src) {
        panic!("parser-subset regression: {label} no longer parses — {e}\nsrc:\n{src}");
    }
}

/// True iff the parser recovered from the snippet (the produced
/// SourceFile has no declarations, indicating a recovery path was
/// taken rather than the intended structure being built). Probes
/// "soft rejection": the parser Ok's but yields nothing usable.
fn expect_parses_but_loses_shape(src: &str, label: &str) {
    match nom_parser::parse_source(src) {
        Ok(sf) => {
            assert!(
                sf.declarations.is_empty() || sf.declarations.len() == 1,
                "{label}: parser produced {} top-level decls; expected 0 or 1 (module only) — \
                 if real structure is now retained, promote the feature",
                sf.declarations.len()
            );
        }
        Err(e) => panic!("{label}: parser returned Err — tighter than before ({e})"),
    }
}

// ── Currently supported (all scaffolds rely on these) ─────────────

#[test]
fn accepts_classifier_plus_module_name() {
    expect_accepts(
        "nom probe\n\nstruct A { x: integer }\n",
        "classifier + module + struct",
    );
}

#[test]
fn accepts_simple_enum_bare_variants() {
    expect_accepts(
        "nom probe\n\nenum Color { Red, Green, Blue }\n",
        "enum with bare variants",
    );
}

#[test]
fn accepts_fn_with_primitive_return() {
    expect_accepts(
        "nom probe\n\nfn zero() -> integer { return 0 }\n",
        "fn -> integer",
    );
}

#[test]
fn accepts_fn_with_struct_param() {
    expect_accepts(
        "nom probe\n\nstruct P { n: integer }\n\nfn pn(p: P) -> integer { return p.n }\n",
        "fn (struct_param) -> integer",
    );
}

#[test]
fn accepts_if_chain_returning_text() {
    // Pattern used throughout the self-host scaffolds' is_known_* helpers.
    expect_accepts(
        "nom probe\n\n\
         fn pick(n: integer) -> text {\n\
         if n == 0 {\n return \"zero\"\n}\n\
         if n == 1 {\n return \"one\"\n}\n\
         return \"other\"\n\
         }\n",
        "if-chain with early returns",
    );
}

// ── Known gaps (aspirational per stdlib/self_host/README.md) ──────
//
// The parser is lenient — it recovers from these and returns Ok with
// warnings, but the produced SourceFile loses shape. When the real
// lowering arrives, these tests flip to `expect_accepts`.

#[test]
fn tuple_return_type_recovers_but_loses_shape() {
    expect_parses_but_loses_shape(
        "nom probe\n\nfn pair() -> (integer, integer) { return (1, 2) }\n",
        "fn -> tuple",
    );
}

#[test]
fn generic_list_type_recovers_but_loses_shape() {
    expect_parses_but_loses_shape(
        "nom probe\n\nfn empty() -> list[integer] { return [] }\n",
        "list[T] generic return",
    );
}
