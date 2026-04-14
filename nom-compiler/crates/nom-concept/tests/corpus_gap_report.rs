//! Diagnostic: print the first N corpus failures per stage so future
//! wedges can target the tallest bars in the dashboard histogram.
//! Observational — never fails.

use nom_concept::stages::run_pipeline_with_grammar;

fn archive_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("research")
        .join(".archive")
        .join("language-analysis")
        .join("14-nom-translation-examples.md")
}

fn baseline_sql_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nom-grammar")
        .join("data")
        .join("baseline.sql")
}

fn extract_v2_blocks(markdown: &str) -> Vec<String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let looks_like_v2_header = lines[i].contains("v2");
        if !looks_like_v2_header {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len() && j < i + 15 {
            if lines[j].trim_start().starts_with("```nomx") {
                let mut k = j + 1;
                let mut buf = String::new();
                while k < lines.len() && !lines[k].trim_start().starts_with("```") {
                    buf.push_str(lines[k]);
                    buf.push('\n');
                    k += 1;
                }
                blocks.push(buf);
                i = k + 1;
                break;
            }
            j += 1;
        }
        if j >= lines.len() || j >= i + 15 {
            i += 1;
        }
    }
    blocks
}

fn open_baseline_grammar() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");
    let sql = std::fs::read_to_string(baseline_sql_path()).expect("baseline.sql exists");
    conn.execute_batch(&sql).expect("import baseline");
    (dir, conn)
}

#[test]
fn print_first_failures_per_stage() {
    let archive = std::fs::read_to_string(archive_path()).expect("read archive");
    let blocks = extract_v2_blocks(&archive);
    let (_dir, conn) = open_baseline_grammar();

    let mut samples: std::collections::BTreeMap<String, Vec<(usize, String, String, String)>> =
        std::collections::BTreeMap::new();
    for (idx, block) in blocks.iter().enumerate() {
        if let Err(err) = run_pipeline_with_grammar(block, &conn) {
            let stage = err.stage.code().to_string();
            let entry = samples.entry(stage).or_default();
            if entry.len() < 3 {
                let head: String = block.lines().take(5).collect::<Vec<_>>().join("\n");
                entry.push((idx, err.reason.to_string(), err.detail.clone(), head));
            }
        }
    }

    println!("=== corpus failure samples (first 3 per stage) ===");
    for (stage, hits) in &samples {
        println!("\n-- {} ({} failures total) --", stage, hits.len());
        for (idx, reason, detail, head) in hits {
            println!(
                "  block #{idx}  reason={reason}\n    detail={detail}\n    head:\n{}",
                head.lines().map(|l| format!("      {l}")).collect::<Vec<_>>().join("\n")
            );
        }
    }
}
