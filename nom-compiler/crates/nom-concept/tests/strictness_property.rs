//! P4 — Strictness proof (Phase E of the blueprint).
//!
//! Claim: every malformed input produces a structured `StageFailure`
//! with a `NOMX-S<N>-<reason>` diag id; the parser never panics;
//! repeating the same input yields the same diagnostic tuple.
//!
//! The property test feeds 256 pseudo-random byte strings (deterministic
//! PRNG so failures reproduce) through the DB-driven pipeline and
//! asserts panic-freedom + diag-id well-formedness + determinism.

use nom_concept::stages::run_pipeline_with_grammar;
use std::panic::AssertUnwindSafe;

/// Lightweight splitmix64 PRNG — deterministic, no external crate.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Generate a pseudo-random ASCII-biased byte string of length ≤ 500.
/// Mixes printable ASCII + whitespace + occasional raw bytes to
/// exercise both "looks like source" and "garbage" paths.
fn make_input(seed: u64, rng: &mut u64) -> String {
    *rng = seed;
    let len = (splitmix64(rng) % 500) as usize;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        let r = splitmix64(rng);
        let c = match r % 10 {
            0 => ' ',
            1 => '\n',
            2 => '.',
            3..=7 => {
                // printable ASCII range
                let b = 0x20u8 + ((r >> 8) as u8 % 0x5E);
                b as char
            }
            _ => ((r >> 16) as u8 % 0x80) as char,
        };
        s.push(c);
    }
    s
}

fn is_valid_diag_id(id: &str) -> bool {
    // `NOMX-S<N>-<reason>` where N is 1..=6 and reason is lowercase
    // ascii with dashes.
    let prefix_ok = id.starts_with("NOMX-S")
        && id.len() > 7
        && id.as_bytes()[6].is_ascii_digit()
        && id.as_bytes()[7] == b'-';
    if !prefix_ok {
        return false;
    }
    let stage_digit = (id.as_bytes()[6] - b'0') as usize;
    if !(1..=6).contains(&stage_digit) {
        return false;
    }
    let reason = &id[8..];
    !reason.is_empty()
        && reason.chars().all(|c| c.is_ascii_lowercase() || c == '-')
}

fn open_baseline_grammar() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");
    let baseline = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nom-grammar")
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline).expect("baseline.sql exists");
    conn.execute_batch(&sql).expect("import baseline");
    (dir, conn)
}

#[test]
fn pipeline_never_panics_on_random_input() {
    let (_dir, conn) = open_baseline_grammar();

    let mut rng = 0xDEADBEEFu64;
    for seed in 0..256u64 {
        let mut local = rng;
        let input = make_input(seed, &mut local);
        rng = local;

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            run_pipeline_with_grammar(&input, &conn)
        }));
        assert!(
            result.is_ok(),
            "parser panicked on seed {seed}, input {input:?}"
        );
    }
}

#[test]
fn failure_diag_ids_are_well_formed_and_reproducible() {
    let (_dir, conn) = open_baseline_grammar();

    let mut rng = 0xCAFEF00Du64;
    let mut failure_count = 0usize;
    for seed in 0..128u64 {
        let mut local = rng;
        let input = make_input(seed, &mut local);
        rng = local;

        let first = run_pipeline_with_grammar(&input, &conn);
        let second = run_pipeline_with_grammar(&input, &conn);

        // Determinism: same input + same DB → same outcome shape.
        match (&first, &second) {
            (Ok(_), Ok(_)) => {}
            (Err(a), Err(b)) => {
                assert_eq!(
                    (a.stage, a.position, &a.reason),
                    (b.stage, b.position, &b.reason),
                    "non-deterministic failure on input {input:?}"
                );
            }
            _ => panic!(
                "non-deterministic Ok/Err divergence on input {input:?}: {first:?} vs {second:?}"
            ),
        }

        if let Err(err) = first {
            failure_count += 1;
            let id = err.diag_id();
            assert!(
                is_valid_diag_id(&id),
                "malformed diag_id {id:?} for input {input:?}"
            );
        }
    }

    // Expect most random bytes to fail — confirm the strictness lane
    // is actually firing rather than silently accepting garbage.
    assert!(
        failure_count > 0,
        "property test saw zero failures over 128 random inputs — strictness may be too loose"
    );
}

#[test]
fn diag_id_validator_rejects_known_bad_shapes() {
    assert!(!is_valid_diag_id(""));
    assert!(!is_valid_diag_id("NOMX-"));
    assert!(!is_valid_diag_id("NOMX-S"));
    assert!(!is_valid_diag_id("NOMX-SA-foo"));
    assert!(!is_valid_diag_id("NOMX-S9-foo"));
    assert!(!is_valid_diag_id("NOMX-S1-"));
    assert!(!is_valid_diag_id("NOMX-S1-UPPER"));
    assert!(is_valid_diag_id("NOMX-S1-unknown-token"));
    assert!(is_valid_diag_id("NOMX-S2-empty-registry"));
    assert!(is_valid_diag_id("NOMX-S5-unknown-quality-name"));
}
