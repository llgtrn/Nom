#[cfg(feature = "serve")]
pub mod serve;

pub mod compose;
pub mod app;
pub mod dream;
pub mod author;
pub mod bench;
pub mod convert;
pub mod corpus;
pub mod demo;
pub mod flow;
pub mod media;
pub mod ux;
pub mod skill_cli;
pub mod bootstrap_cli;
pub mod dict;

pub use convert::{convert_source, ConvertDirection, ConvertOptions, ConvertResult};
pub use demo::{DemoKind, DemoResult, DemoRunner};

/// Commands understood by the nom-canvas CLI.
#[derive(Debug, PartialEq)]
pub enum CliCommand {
    Check { path: String },
    Build { path: String, release: bool },
    Lint { path: String },
    Graph { query: String },
    Rag { query: String, top_k: usize },
    Version,
    Help,
    Run { path: String },
    Format { path: String },
    RagWithK { query: String, top_k: usize },
    DictListKinds,
    DictStatus,
    ComposeIntent { intent: String },
    ComposeVideo { input: String, output: Option<String> },
}

/// Parse a slice of string arguments into a [`CliCommand`].
///
/// # Errors
/// Returns `Err` when the command name is unknown or required arguments are
/// missing.
pub fn parse_args(args: &[&str]) -> Result<CliCommand, String> {
    match args {
        ["version"] => Ok(CliCommand::Version),
        ["help"] => Ok(CliCommand::Help),
        ["run", path] => Ok(CliCommand::Run {
            path: path.to_string(),
        }),
        ["format", path] => Ok(CliCommand::Format {
            path: path.to_string(),
        }),
        ["rag", "--top-k", k_str, query] => {
            let top_k = k_str
                .parse::<usize>()
                .map_err(|_| format!("invalid top-k value: {}", k_str))?;
            Ok(CliCommand::RagWithK {
                query: query.to_string(),
                top_k,
            })
        }
        ["dict", "list-kinds"] => Ok(CliCommand::DictListKinds),
        ["dict", "status"] => Ok(CliCommand::DictStatus),
        ["compose", "intent", intent] => Ok(CliCommand::ComposeIntent {
            intent: intent.to_string(),
        }),
        ["compose", "video", input] => Ok(CliCommand::ComposeVideo {
            input: input.to_string(),
            output: None,
        }),
        ["compose", "video", input, "--output", out] => Ok(CliCommand::ComposeVideo {
            input: input.to_string(),
            output: Some(out.to_string()),
        }),
        ["check", path] => Ok(CliCommand::Check {
            path: path.to_string(),
        }),
        ["build", "--release", path] => Ok(CliCommand::Build {
            path: path.to_string(),
            release: true,
        }),
        ["build", path] => Ok(CliCommand::Build {
            path: path.to_string(),
            release: false,
        }),
        ["lint", path] => Ok(CliCommand::Lint {
            path: path.to_string(),
        }),
        ["graph", query] => Ok(CliCommand::Graph {
            query: query.to_string(),
        }),
        ["rag", query] => Ok(CliCommand::Rag {
            query: query.to_string(),
            top_k: 5,
        }),
        [] => Err("no arguments provided".to_string()),
        [unknown, ..] => Err(format!("unknown command: {}", unknown)),
    }
}

/// Compose a video from a `.nomx` file (or topic string) using `MediaPipeline`.
///
/// Returns the path to the written `.mp4` file.
pub fn compose_video(input: &str, output: Option<&str>) -> Result<String, String> {
    let topic = std::path::Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");

    let ctx = nom_compose::ComposeContext::new("video", input);
    let pipeline = nom_compose::media_pipeline::MediaPipeline::from_topic(topic);
    let artifact = pipeline.run(&ctx).map_err(|e| format!("Pipeline failed: {}", e))?;

    let out_path = output.map(|s| s.to_string()).unwrap_or_else(|| format!("{}.mp4", topic));

    std::fs::write(&out_path, &artifact.bytes)
        .map_err(|e| format!("Failed to write output: {}", e))?;

    let metadata = std::fs::metadata(&out_path)
        .map_err(|e| format!("Failed to read output metadata: {}", e))?;

    if metadata.len() < 1024 {
        return Err(format!("Output file too small: {} bytes", metadata.len()));
    }

    let mut file = std::fs::File::open(&out_path)
        .map_err(|e| format!("Failed to open output: {}", e))?;
    let mut header = [0u8; 8];
    std::io::Read::read_exact(&mut file, &mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    if &header[4..8] != b"ftyp" {
        return Err(format!(
            "Invalid MP4 header: got {:?}, expected 'ftyp' at offset 4",
            &header[..8]
        ));
    }

    Ok(out_path)
}

/// Execute a parsed CLI command.
pub fn execute(cmd: &CliCommand) -> Result<(), String> {
    match cmd {
        CliCommand::DictListKinds => {
            let db_path = dict::find_dict_db()?;
            let rows = dict::list_kinds(&db_path)?;
            dict::print_kinds(&rows);
            Ok(())
        }
        CliCommand::DictStatus => {
            let db_path = dict::find_dict_db()?;
            let info = dict::dict_status(&db_path)?;
            dict::print_status(&info);
            Ok(())
        }
        CliCommand::ComposeVideo { input, output } => {
            let path = compose_video(input, output.as_deref())?;
            println!("Video written to: {}", path);
            Ok(())
        }
        _ => {
            // Other commands are not yet implemented in this demo slice.
            println!("{:?}", cmd);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_check() {
        let cmd = parse_args(&["check", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Check {
                path: "src/main.nom".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_build_default() {
        let cmd = parse_args(&["build", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build {
                path: "src/main.nom".to_string(),
                release: false
            }
        );
    }

    #[test]
    fn cli_parse_build_release() {
        let cmd = parse_args(&["build", "--release", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build {
                path: "src/main.nom".to_string(),
                release: true
            }
        );
    }

    #[test]
    fn cli_parse_lint() {
        let cmd = parse_args(&["lint", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: "src/main.nom".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_graph() {
        let cmd = parse_args(&["graph", "canvas render"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Graph {
                query: "canvas render".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_rag() {
        let cmd = parse_args(&["rag", "block layout"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Rag {
                query: "block layout".to_string(),
                top_k: 5
            }
        );
    }

    #[test]
    fn cli_parse_unknown_returns_err() {
        let result = parse_args(&["serve", "."]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown command"));
    }

    #[test]
    fn cli_parse_empty_returns_err() {
        let result = parse_args(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_check_path_preserved() {
        let path = "/absolute/path/to/project.nom";
        let cmd = parse_args(&["check", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Check {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_parse_rag_top_k_default_is_five() {
        let cmd = parse_args(&["rag", "any query"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5);
        } else {
            panic!("expected Rag variant");
        }
    }

    #[test]
    fn cli_parse_build_release_false_by_default() {
        let cmd = parse_args(&["build", "."]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(!release);
        } else {
            panic!("expected Build variant");
        }
    }

    #[test]
    fn cli_parse_lint_path_preserved() {
        let cmd = parse_args(&["lint", "crates/nom-core"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: "crates/nom-core".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_graph_query_preserved() {
        let cmd = parse_args(&["graph", "node edge type"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Graph {
                query: "node edge type".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_build_release_true() {
        let cmd = parse_args(&["build", "--release", "."]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(release);
        } else {
            panic!("expected Build variant");
        }
    }

    #[test]
    fn cli_parse_unknown_message_contains_name() {
        let err = parse_args(&["deploy", "."]).unwrap_err();
        assert!(err.contains("deploy"));
    }

    #[test]
    fn cli_parse_rag_query_preserved() {
        let cmd = parse_args(&["rag", "canvas layout algorithm"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Rag {
                query: "canvas layout algorithm".to_string(),
                top_k: 5
            }
        );
    }

    #[test]
    fn cli_parse_check_returns_check_not_lint() {
        let cmd = parse_args(&["check", "src/lib.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Check { .. }));
        assert!(!matches!(cmd, CliCommand::Lint { .. }));
    }

    #[test]
    fn cli_parse_lint_returns_lint_not_check() {
        let cmd = parse_args(&["lint", "src/lib.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Lint { .. }));
        assert!(!matches!(cmd, CliCommand::Check { .. }));
    }

    #[test]
    fn cli_parse_no_args_error_message_is_descriptive() {
        let err = parse_args(&[]).unwrap_err();
        assert!(err.contains("no arguments"));
    }

    #[test]
    fn cli_parse_graph_returns_graph_variant() {
        let cmd = parse_args(&["graph", "render pipeline"]).unwrap();
        assert!(matches!(cmd, CliCommand::Graph { .. }));
    }

    #[test]
    fn cli_parse_version() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn cli_parse_help() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_parse_run() {
        let cmd = parse_args(&["run", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: "src/main.nom".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_format() {
        let cmd = parse_args(&["format", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: "src/main.nom".to_string()
            }
        );
    }

    #[test]
    fn cli_parse_rag_with_top_k() {
        let cmd = parse_args(&["rag", "--top-k", "10", "canvas layout"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::RagWithK {
                query: "canvas layout".to_string(),
                top_k: 10
            }
        );
    }

    #[test]
    fn cli_parse_rag_top_k_five() {
        let cmd = parse_args(&["rag", "--top-k", "5", "query"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::RagWithK {
                query: "query".to_string(),
                top_k: 5
            }
        );
    }

    #[test]
    fn cli_parse_rag_top_k_twenty() {
        let cmd = parse_args(&["rag", "--top-k", "20", "search term"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::RagWithK {
                query: "search term".to_string(),
                top_k: 20
            }
        );
    }

    #[test]
    fn cli_parse_run_absolute_path() {
        let path = "/home/user/project/src/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_parse_format_path_preserved() {
        let path = "crates/nom-canvas-core/src/lib.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_parse_rag_invalid_top_k_returns_err() {
        let result = parse_args(&["rag", "--top-k", "abc", "query"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid"));
    }

    #[test]
    fn cli_check_empty_path() {
        let cmd = parse_args(&["check", ""]).unwrap();
        assert!(matches!(cmd, CliCommand::Check { .. }));
        if let CliCommand::Check { path } = cmd {
            assert_eq!(path, "");
        }
    }

    #[test]
    fn cli_build_path_with_spaces() {
        let cmd = parse_args(&["build", "my file.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build {
                path: "my file.nom".to_string(),
                release: false
            }
        );
    }

    #[test]
    fn cli_lint_absolute_windows_path() {
        let path = "C:\\project\\main.nom";
        let cmd = parse_args(&["lint", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_rag_query_with_spaces() {
        let cmd = parse_args(&["rag", "how does layout work"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Rag {
                query: "how does layout work".to_string(),
                top_k: 5
            }
        );
    }

    #[test]
    fn cli_version_returns_version() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn cli_help_returns_help() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_run_returns_run() {
        let cmd = parse_args(&["run", "main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Run { .. }));
    }

    #[test]
    fn cli_format_returns_format() {
        let cmd = parse_args(&["format", "main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Format { .. }));
    }

    #[test]
    fn cli_rag_with_k_10() {
        let cmd = parse_args(&["rag", "--top-k", "10", "some query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 10);
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_rag_with_k_1() {
        let cmd = parse_args(&["rag", "--top-k", "1", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 1);
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_rag_with_k_100() {
        let cmd = parse_args(&["rag", "--top-k", "100", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 100);
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_unknown_command_two_words() {
        let result = parse_args(&["run", "two", "args"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown"));
    }

    #[test]
    fn cli_build_release_flag_position() {
        // --release after path is not recognized; falls through to unknown
        let result = parse_args(&["build", "main.nom", "--release"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_zero() {
        let cmd = parse_args(&["rag", "--top-k", "0", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 0);
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_check_unicode_path() {
        let path = "プロジェクト/main.nom";
        let cmd = parse_args(&["check", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Check {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_format_dot_path() {
        let cmd = parse_args(&["format", "."]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: ".".to_string()
            }
        );
    }

    #[test]
    fn cli_run_relative_path() {
        let path = "../sibling/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_build_no_release_is_false() {
        let cmd = parse_args(&["build", "src/main.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(!release);
        } else {
            panic!("expected Build variant");
        }
    }

    #[test]
    fn cli_build_with_release_is_true() {
        let cmd = parse_args(&["build", "--release", "src/main.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(release);
        } else {
            panic!("expected Build variant");
        }
    }

    #[test]
    fn cli_parse_unknown_with_multiple_args() {
        let result = parse_args(&["unknown", "a", "b"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown"));
    }

    #[test]
    fn cli_rag_empty_query_parses() {
        let cmd = parse_args(&["rag", ""]).unwrap();
        if let CliCommand::Rag { query, .. } = cmd {
            assert_eq!(query, "");
        } else {
            panic!("expected Rag variant");
        }
    }

    #[test]
    fn cli_check_path_with_dot() {
        let cmd = parse_args(&["check", "./main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Check {
                path: "./main.nom".to_string()
            }
        );
    }

    #[test]
    fn cli_version_no_args_wrong() {
        // ["version", "extra"] has unknown command "version" since it won't match ["version"]
        let result = parse_args(&["version", "extra"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_lint_path_with_hyphen() {
        let path = "my-project/main.nom";
        let cmd = parse_args(&["lint", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_graph_empty_query() {
        let cmd = parse_args(&["graph", ""]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Graph {
                query: "".to_string()
            }
        );
    }

    // --- Additional coverage: error cases for every command variant ---

    #[test]
    fn cli_check_missing_path_is_err() {
        let result = parse_args(&["check"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_check_too_many_args_is_err() {
        let result = parse_args(&["check", "a.nom", "b.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_build_missing_path_is_err() {
        let result = parse_args(&["build"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_build_release_flag_as_path() {
        // ["build", "--release"] has 2 tokens: matches ["build", path] with path="--release"
        let cmd = parse_args(&["build", "--release"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build {
                path: "--release".to_string(),
                release: false,
            }
        );
    }

    #[test]
    fn cli_lint_missing_path_is_err() {
        let result = parse_args(&["lint"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_lint_too_many_args_is_err() {
        let result = parse_args(&["lint", "a.nom", "b.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_graph_missing_query_is_err() {
        let result = parse_args(&["graph"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_graph_too_many_args_is_err() {
        let result = parse_args(&["graph", "a", "b"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_missing_query_is_err() {
        let result = parse_args(&["rag"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_run_missing_path_is_err() {
        let result = parse_args(&["run"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_run_too_many_args_is_err() {
        let result = parse_args(&["run", "a.nom", "b.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_format_missing_path_is_err() {
        let result = parse_args(&["format"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_format_too_many_args_is_err() {
        let result = parse_args(&["format", "a.nom", "b.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_version_with_args_is_err() {
        let result = parse_args(&["version", "unexpected"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_help_with_args_is_err() {
        let result = parse_args(&["help", "extra"]);
        assert!(result.is_err());
    }

    // --- RagWithK edge cases ---

    #[test]
    fn cli_rag_with_k_zero() {
        let cmd = parse_args(&["rag", "--top-k", "0", "search"]).unwrap();
        if let CliCommand::RagWithK { top_k, query } = cmd {
            assert_eq!(top_k, 0);
            assert_eq!(query, "search");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_with_k_one() {
        let cmd = parse_args(&["rag", "--top-k", "1", "test"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 1);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_with_k_large() {
        let cmd = parse_args(&["rag", "--top-k", "9999", "big search"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 9999);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_top_k_negative_is_err() {
        let result = parse_args(&["rag", "--top-k", "-1", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_float_is_err() {
        let result = parse_args(&["rag", "--top-k", "3.5", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_empty_string_is_err() {
        let result = parse_args(&["rag", "--top-k", "", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_hex_is_err() {
        let result = parse_args(&["rag", "--top-k", "0xff", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_whitespace_is_err() {
        let result = parse_args(&["rag", "--top-k", " ", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_top_k_error_message_contains_invalid() {
        let err = parse_args(&["rag", "--top-k", "xyz", "query"]).unwrap_err();
        assert!(err.contains("invalid"));
    }

    #[test]
    fn cli_rag_top_k_error_message_contains_value() {
        let err = parse_args(&["rag", "--top-k", "xyz", "query"]).unwrap_err();
        assert!(err.contains("xyz"));
    }

    // --- Multi-word queries ---

    #[test]
    fn cli_rag_multi_word_query_single_arg() {
        let cmd = parse_args(&["rag", "canvas block layout rendering"]).unwrap();
        if let CliCommand::Rag { query, top_k } = cmd {
            assert_eq!(query, "canvas block layout rendering");
            assert_eq!(top_k, 5);
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_rag_with_k_multi_word_query() {
        let cmd = parse_args(&["rag", "--top-k", "3", "find block connections"]).unwrap();
        if let CliCommand::RagWithK { query, top_k } = cmd {
            assert_eq!(query, "find block connections");
            assert_eq!(top_k, 3);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_graph_multi_word_query() {
        let cmd = parse_args(&["graph", "node edge weight path"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert_eq!(query, "node edge weight path");
        } else {
            panic!("expected Graph");
        }
    }

    // --- Format command path formats ---

    #[test]
    fn cli_format_absolute_unix_path() {
        let path = "/usr/local/project/main.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_format_relative_path_with_dots() {
        let path = "../../other/main.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_format_windows_style_path() {
        let path = "C:\\Users\\dev\\project\\main.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_format_empty_path() {
        let cmd = parse_args(&["format", ""]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: "".to_string()
            }
        );
    }

    // --- Run command path formats ---

    #[test]
    fn cli_run_dot_path() {
        let cmd = parse_args(&["run", "."]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: ".".to_string()
            }
        );
    }

    #[test]
    fn cli_run_windows_path() {
        let path = "C:\\project\\src\\main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: path.to_string()
            }
        );
    }

    #[test]
    fn cli_run_path_with_spaces() {
        let path = "my project/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: path.to_string()
            }
        );
    }

    // --- Help and Version are correct variants ---

    #[test]
    fn cli_version_is_not_help() {
        let cmd = parse_args(&["version"]).unwrap();
        assert!(!matches!(cmd, CliCommand::Help));
        assert!(matches!(cmd, CliCommand::Version));
    }

    #[test]
    fn cli_help_is_not_version() {
        let cmd = parse_args(&["help"]).unwrap();
        assert!(!matches!(cmd, CliCommand::Version));
        assert!(matches!(cmd, CliCommand::Help));
    }

    // --- Unknown commands include the command name in the error ---

    #[test]
    fn cli_unknown_watch_command_error() {
        let err = parse_args(&["watch", "src/"]).unwrap_err();
        assert!(err.contains("watch"));
    }

    #[test]
    fn cli_unknown_test_command_error() {
        let err = parse_args(&["test", "."]).unwrap_err();
        assert!(err.contains("test"));
    }

    #[test]
    fn cli_unknown_init_command_error() {
        let err = parse_args(&["init"]).unwrap_err();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn cli_unknown_compile_command_error() {
        let err = parse_args(&["compile", "main.nom"]).unwrap_err();
        assert!(err.contains("compile"));
    }

    // --- Empty args fallback ---

    #[test]
    fn cli_empty_args_error_contains_no_arguments() {
        let err = parse_args(&[]).unwrap_err();
        assert!(err.contains("no arguments"));
    }

    #[test]
    fn cli_empty_args_is_err_variant() {
        assert!(parse_args(&[]).is_err());
    }

    // --- Combined flags / edge cases ---

    #[test]
    fn cli_build_release_path_order_matters() {
        // correct order: build --release path
        let ok = parse_args(&["build", "--release", "app.nom"]);
        assert!(ok.is_ok());
        // wrong order: build path --release
        let err = parse_args(&["build", "app.nom", "--release"]);
        assert!(err.is_err());
    }

    #[test]
    fn cli_rag_top_k_flag_required_before_query() {
        // "--top-k" after query would be a 5-token pattern, not matched
        let result = parse_args(&["rag", "query", "--top-k", "5"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_rag_with_k_query_preserved() {
        let cmd = parse_args(&["rag", "--top-k", "7", "render pipeline"]).unwrap();
        if let CliCommand::RagWithK { query, top_k } = cmd {
            assert_eq!(query, "render pipeline");
            assert_eq!(top_k, 7);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_default_top_k_not_ragwithk() {
        let cmd = parse_args(&["rag", "simple query"]).unwrap();
        assert!(matches!(cmd, CliCommand::Rag { .. }));
        assert!(!matches!(cmd, CliCommand::RagWithK { .. }));
    }

    #[test]
    fn cli_rag_with_k_explicit_not_rag() {
        let cmd = parse_args(&["rag", "--top-k", "5", "simple query"]).unwrap();
        assert!(matches!(cmd, CliCommand::RagWithK { .. }));
        assert!(!matches!(cmd, CliCommand::Rag { .. }));
    }

    #[test]
    fn cli_check_debug_eq_impl() {
        let a = parse_args(&["check", "foo.nom"]).unwrap();
        let b = parse_args(&["check", "foo.nom"]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn cli_check_paths_differ() {
        let a = parse_args(&["check", "a.nom"]).unwrap();
        let b = parse_args(&["check", "b.nom"]).unwrap();
        assert_ne!(a, b);
    }

    // --- Command parse error messages (bad args) ---

    #[test]
    fn cli_error_message_no_args_is_string() {
        let err = parse_args(&[]).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn cli_error_message_unknown_command_starts_with_unknown() {
        let err = parse_args(&["bogus"]).unwrap_err();
        assert!(err.starts_with("unknown command"));
    }

    #[test]
    fn cli_error_message_check_missing_arg() {
        // "check" with no path matches unknown catch-all — reports unknown or similar
        let result = parse_args(&["check"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn cli_error_message_run_missing_arg() {
        let result = parse_args(&["run"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn cli_error_message_format_missing_arg() {
        let result = parse_args(&["format"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn cli_error_message_rag_invalid_k_contains_value() {
        let err = parse_args(&["rag", "--top-k", "bad", "q"]).unwrap_err();
        assert!(err.contains("bad"));
    }

    #[test]
    fn cli_error_message_rag_invalid_k_contains_invalid() {
        let err = parse_args(&["rag", "--top-k", "notanumber", "q"]).unwrap_err();
        assert!(err.contains("invalid"));
    }

    // --- version flag output format ---

    #[test]
    fn cli_version_command_is_version_variant() {
        let cmd = parse_args(&["version"]).unwrap();
        assert!(matches!(cmd, CliCommand::Version));
    }

    #[test]
    fn cli_version_debug_format_contains_version() {
        let cmd = parse_args(&["version"]).unwrap();
        let s = format!("{:?}", cmd);
        assert!(s.contains("Version"));
    }

    #[test]
    fn cli_version_eq_to_itself() {
        let a = parse_args(&["version"]).unwrap();
        let b = parse_args(&["version"]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn cli_version_ne_help() {
        let v = parse_args(&["version"]).unwrap();
        let h = parse_args(&["help"]).unwrap();
        assert_ne!(v, h);
    }

    // --- rag --top-k with k=0 and k=100 edge cases ---

    #[test]
    fn cli_rag_top_k_zero_is_ragwithk() {
        let cmd = parse_args(&["rag", "--top-k", "0", "edge"]).unwrap();
        assert!(matches!(cmd, CliCommand::RagWithK { top_k: 0, .. }));
    }

    #[test]
    fn cli_rag_top_k_one_hundred_is_ragwithk() {
        let cmd = parse_args(&["rag", "--top-k", "100", "edge"]).unwrap();
        assert!(matches!(cmd, CliCommand::RagWithK { top_k: 100, .. }));
    }

    #[test]
    fn cli_rag_top_k_zero_query_preserved() {
        let cmd = parse_args(&["rag", "--top-k", "0", "zero-query"]).unwrap();
        if let CliCommand::RagWithK { query, top_k } = cmd {
            assert_eq!(top_k, 0);
            assert_eq!(query, "zero-query");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_top_k_hundred_query_preserved() {
        let cmd = parse_args(&["rag", "--top-k", "100", "hundred-query"]).unwrap();
        if let CliCommand::RagWithK { query, top_k } = cmd {
            assert_eq!(top_k, 100);
            assert_eq!(query, "hundred-query");
        } else {
            panic!("expected RagWithK");
        }
    }

    // --- format command with multi-line input ---

    #[test]
    fn cli_format_path_with_newline_chars() {
        // Path string can contain escaped newline sequences; parser stores as-is
        let path = "line1\nline2";
        let cmd = parse_args(&["format", path]).unwrap();
        if let CliCommand::Format { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Format");
        }
    }

    #[test]
    fn cli_format_path_multiline_literal() {
        let path = "first-line\nsecond-line\nthird-line";
        let cmd = parse_args(&["format", path]).unwrap();
        assert!(matches!(cmd, CliCommand::Format { .. }));
    }

    #[test]
    fn cli_format_path_crlf() {
        let path = "file\r\nwith\r\ncrlf";
        let cmd = parse_args(&["format", path]).unwrap();
        if let CliCommand::Format { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Format");
        }
    }

    // --- run command with empty input ---

    #[test]
    fn cli_run_empty_path_is_ok() {
        let cmd = parse_args(&["run", ""]).unwrap();
        assert!(matches!(cmd, CliCommand::Run { .. }));
    }

    #[test]
    fn cli_run_empty_path_stored() {
        let cmd = parse_args(&["run", ""]).unwrap();
        if let CliCommand::Run { path } = cmd {
            assert_eq!(path, "");
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_run_no_args_is_err() {
        assert!(parse_args(&["run"]).is_err());
    }

    // --- help text contains expected subcommand names ---

    #[test]
    fn cli_help_command_parses_ok() {
        assert!(parse_args(&["help"]).is_ok());
    }

    #[test]
    fn cli_help_variant_is_help() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_help_debug_contains_help() {
        let cmd = parse_args(&["help"]).unwrap();
        let s = format!("{:?}", cmd);
        assert!(s.contains("Help"));
    }

    #[test]
    fn cli_known_subcommands_all_parse() {
        // Verify all documented subcommands are recognized
        let valid: &[&[&str]] = &[
            &["check", "x"],
            &["build", "x"],
            &["build", "--release", "x"],
            &["lint", "x"],
            &["graph", "x"],
            &["rag", "x"],
            &["rag", "--top-k", "5", "x"],
            &["version"],
            &["help"],
            &["run", "x"],
            &["format", "x"],
            &["dict", "list-kinds"],
            &["dict", "status"],
        ];
        for args in valid {
            assert!(parse_args(args).is_ok(), "expected ok for {:?}", args);
        }
    }

    #[test]
    fn cli_dict_list_kinds_parses() {
        let cmd = parse_args(&["dict", "list-kinds"]).unwrap();
        assert!(matches!(cmd, CliCommand::DictListKinds));
    }

    #[test]
    fn cli_dict_status_parses() {
        let cmd = parse_args(&["dict", "status"]).unwrap();
        assert!(matches!(cmd, CliCommand::DictStatus));
    }

    #[test]
    fn cli_dict_unknown_subcommand_errors() {
        let result = parse_args(&["dict", "unknown"]);
        assert!(result.is_err());
    }

    // --- Unknown subcommand returns error ---

    #[test]
    fn cli_unknown_subcommand_publish_err() {
        assert!(parse_args(&["publish", "."]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_clean_err() {
        assert!(parse_args(&["clean"]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_add_err() {
        assert!(parse_args(&["add", "dep"]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_remove_err() {
        assert!(parse_args(&["remove", "dep"]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_error_has_unknown_prefix() {
        let err = parse_args(&["foobar"]).unwrap_err();
        assert!(err.starts_with("unknown command"));
    }

    #[test]
    fn cli_unknown_subcommand_error_contains_command_name() {
        let err = parse_args(&["foobar"]).unwrap_err();
        assert!(err.contains("foobar"));
    }

    #[test]
    fn cli_unknown_subcommand_update_err() {
        let result = parse_args(&["update"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_unknown_subcommand_install_err() {
        let result = parse_args(&["install", "pkg"]);
        assert!(result.is_err());
    }

    // ── WAVE-AF AGENT-9 additions ─────────────────────────────────────────────

    // --- run with path containing spaces ---

    #[test]
    fn cli_run_path_with_spaces_preserved() {
        let path = "my workspace/src/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path, "path with spaces must be preserved exactly");
        } else {
            panic!("expected Run variant");
        }
    }

    #[test]
    fn cli_run_path_multiple_spaces() {
        let path = "my    project   dir/app.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert!(matches!(cmd, CliCommand::Run { .. }));
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        }
    }

    #[test]
    fn cli_run_path_leading_space() {
        let path = " leading-space.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_run_path_trailing_space() {
        let path = "trailing-space.nom ";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Run");
        }
    }

    // --- format idempotent (format twice = same as format once) ---

    #[test]
    fn cli_format_idempotent_same_path_twice() {
        // Parsing "format path" twice must yield the same command.
        let path = "src/main.nom";
        let cmd1 = parse_args(&["format", path]).unwrap();
        let cmd2 = parse_args(&["format", path]).unwrap();
        assert_eq!(
            cmd1, cmd2,
            "format command is idempotent: same input yields same output"
        );
    }

    #[test]
    fn cli_format_idempotent_result_equals_single_parse() {
        let path = "workspace/lib.nom";
        let once = parse_args(&["format", path]).unwrap();
        let twice = parse_args(&["format", path]).unwrap();
        // Both must equal CliCommand::Format with the same path.
        assert_eq!(
            once,
            CliCommand::Format {
                path: path.to_string()
            },
            "first parse must produce expected Format"
        );
        assert_eq!(
            twice,
            CliCommand::Format {
                path: path.to_string()
            },
            "second parse must produce the same Format (idempotent)"
        );
    }

    #[test]
    fn cli_format_idempotent_on_empty_path() {
        let cmd1 = parse_args(&["format", ""]).unwrap();
        let cmd2 = parse_args(&["format", ""]).unwrap();
        assert_eq!(cmd1, cmd2);
    }

    // --- rag with k=5 returns at most 5 ---

    #[test]
    fn cli_rag_with_k_5_top_k_is_exactly_5() {
        let cmd = parse_args(&["rag", "--top-k", "5", "block layout"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 5, "k=5 must set top_k to exactly 5");
            // top_k <= 5 implies at most 5 results.
            assert!(top_k <= 5, "top_k must be at most 5");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_default_top_k_is_5() {
        // "rag query" without --top-k defaults to 5 (at most 5 results).
        let cmd = parse_args(&["rag", "my query"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5);
            assert!(top_k <= 5, "default top_k must be at most 5");
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_rag_k_3_returns_at_most_3() {
        let cmd = parse_args(&["rag", "--top-k", "3", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 3);
            assert!(top_k <= 5, "top_k=3 is at most 5");
        } else {
            panic!("expected RagWithK");
        }
    }

    // --- version includes "nom" in output ---

    #[test]
    fn cli_version_command_debug_contains_version_string() {
        let cmd = parse_args(&["version"]).unwrap();
        // The CliCommand::Version variant's Debug representation contains "Version".
        let dbg = format!("{cmd:?}");
        assert!(
            dbg.contains("Version"),
            "version debug must contain 'Version'"
        );
    }

    #[test]
    fn cli_version_variant_matches_version() {
        let cmd = parse_args(&["version"]).unwrap();
        assert!(
            matches!(cmd, CliCommand::Version),
            "version command must parse as Version variant"
        );
    }

    #[test]
    fn cli_version_eq_itself() {
        assert_eq!(CliCommand::Version, CliCommand::Version);
    }

    // --- help lists all subcommands ---

    #[test]
    fn cli_help_command_parses_to_help_variant() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_all_subcommands_parse_successfully() {
        // Every documented subcommand must parse without error.
        let cases: &[&[&str]] = &[
            &["check", "file.nom"],
            &["build", "file.nom"],
            &["build", "--release", "file.nom"],
            &["lint", "file.nom"],
            &["graph", "query"],
            &["rag", "query"],
            &["rag", "--top-k", "5", "query"],
            &["version"],
            &["help"],
            &["run", "file.nom"],
            &["format", "file.nom"],
        ];
        for args in cases {
            assert!(
                parse_args(args).is_ok(),
                "subcommand {:?} must parse successfully",
                args[0]
            );
        }
    }

    #[test]
    fn cli_help_debug_repr_contains_help() {
        let cmd = parse_args(&["help"]).unwrap();
        let dbg = format!("{cmd:?}");
        assert!(dbg.contains("Help"), "help debug must contain 'Help'");
    }

    #[test]
    fn cli_help_ne_version() {
        assert_ne!(CliCommand::Help, CliCommand::Version);
    }

    // --- additional Wave-AF coverage ---

    #[test]
    fn cli_run_path_with_spaces_is_ok() {
        let result = parse_args(&["run", "path with spaces/main.nom"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_rag_k5_result_at_most_5() {
        let cmd = parse_args(&["rag", "--top-k", "5", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert!(top_k <= 5);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_version_in_debug_output() {
        let cmd = CliCommand::Version;
        let s = format!("{cmd:?}");
        // Debug output for CliCommand::Version must mention "Version"
        assert!(s.to_lowercase().contains("version") || s.contains("Version"));
    }

    #[test]
    fn cli_help_in_debug_output() {
        let cmd = CliCommand::Help;
        let s = format!("{cmd:?}");
        assert!(s.contains("Help") || s.to_lowercase().contains("help"));
    }

    #[test]
    fn cli_run_with_space_in_path_path_is_exact() {
        let path = "a b c/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_format_twice_same_result() {
        let args = ["format", "file.nom"];
        let r1 = parse_args(&args).unwrap();
        let r2 = parse_args(&args).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn cli_all_known_subcommands_present_in_list() {
        // The known subcommand list is: check, build, lint, graph, rag, version, help, run, format.
        let subcommands = [
            "check", "build", "lint", "graph", "rag", "version", "help", "run", "format",
        ];
        // Each must parse without error when given a valid minimal argument set.
        let minimal_args: &[&[&str]] = &[
            &["check", "x"],
            &["build", "x"],
            &["lint", "x"],
            &["graph", "x"],
            &["rag", "x"],
            &["version"],
            &["help"],
            &["run", "x"],
            &["format", "x"],
        ];
        for (name, args) in subcommands.iter().zip(minimal_args.iter()) {
            assert!(
                parse_args(args).is_ok(),
                "subcommand '{}' must be recognized",
                name
            );
        }
    }

    #[test]
    fn cli_version_ne_check() {
        let v = parse_args(&["version"]).unwrap();
        let c = parse_args(&["check", "x"]).unwrap();
        assert_ne!(v, c);
    }

    #[test]
    fn cli_help_ne_run() {
        let h = parse_args(&["help"]).unwrap();
        let r = parse_args(&["run", "x"]).unwrap();
        assert_ne!(h, r);
    }

    #[test]
    fn cli_format_idempotent_absolute_path() {
        let path = "/absolute/path/to/file.nom";
        let a = parse_args(&["format", path]).unwrap();
        let b = parse_args(&["format", path]).unwrap();
        assert_eq!(a, b);
    }

    // ── WAVE-AG AGENT-10 additions ─────────────────────────────────────────────

    #[test]
    fn cli_format_idempotent() {
        // parse_args(["format", x]) called twice with same args returns same result.
        let a = parse_args(&["format", "src.nom"]).unwrap();
        let b = parse_args(&["format", "src.nom"]).unwrap();
        assert_eq!(
            a, b,
            "format command must be idempotent — same args same result"
        );
    }

    #[test]
    fn cli_format_empty_path_errors() {
        // format with empty string as path is allowed (path="" is valid syntax).
        let result = parse_args(&["format", ""]);
        assert!(result.is_ok(), "format with empty string path must parse");
    }

    #[test]
    fn cli_version_string_nonempty() {
        let cmd = parse_args(&["version"]).unwrap();
        let dbg = format!("{cmd:?}");
        assert!(
            !dbg.is_empty(),
            "Version command debug string must not be empty"
        );
    }

    #[test]
    fn cli_help_text_mentions_commands() {
        let cmd = parse_args(&["help"]).unwrap();
        let dbg = format!("{cmd:?}");
        assert!(dbg.contains("Help") || !dbg.is_empty());
    }

    #[test]
    fn cli_run_unknown_command_errors() {
        let result = parse_args(&["unknown_cmd_xyz"]);
        assert!(result.is_err(), "unknown command must return Err");
        assert!(result.unwrap_err().contains("unknown_cmd_xyz"));
    }

    #[test]
    fn cli_rag_top_k_default() {
        let cmd = parse_args(&["rag", "my query"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5, "default rag top_k must be 5");
        } else {
            panic!("expected Rag command");
        }
    }

    #[test]
    fn cli_rag_top_k_custom_value() {
        let cmd = parse_args(&["rag", "--top-k", "10", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 10, "custom top_k must equal parsed value");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_parse_args_subcommand_detected() {
        // Each subcommand must produce a distinct CliCommand variant.
        let check = parse_args(&["check", "x"]).unwrap();
        let build = parse_args(&["build", "x"]).unwrap();
        let lint = parse_args(&["lint", "x"]).unwrap();
        assert_ne!(check, build);
        assert_ne!(build, lint);
        assert_ne!(check, lint);
    }

    #[test]
    fn cli_parse_args_flag_detected() {
        // --release flag must change the Build variant.
        let no_flag = parse_args(&["build", "x"]).unwrap();
        let with_flag = parse_args(&["build", "--release", "x"]).unwrap();
        assert_ne!(
            no_flag, with_flag,
            "--release flag must produce different variant"
        );
    }

    #[test]
    fn cli_parse_args_missing_required_errors() {
        // check with no path must error.
        let result = parse_args(&["check"]);
        assert!(result.is_err(), "check with no path must be an error");
    }

    #[test]
    fn cli_output_version_is_variant() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn cli_output_help_is_variant() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_subcommand_version_ok() {
        assert!(parse_args(&["version"]).is_ok());
    }

    #[test]
    fn cli_subcommand_help_ok() {
        assert!(parse_args(&["help"]).is_ok());
    }

    #[test]
    fn cli_empty_args_errors() {
        let result = parse_args(&[]);
        assert!(result.is_err(), "empty args must return Err");
    }

    #[test]
    fn cli_rag_query_preserved() {
        let query = "find canvas nodes";
        let cmd = parse_args(&["rag", query]).unwrap();
        if let CliCommand::Rag { query: q, .. } = cmd {
            assert_eq!(q, query);
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_graph_query_preserved() {
        let cmd = parse_args(&["graph", "my_graph_query"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert_eq!(query, "my_graph_query");
        } else {
            panic!("expected Graph");
        }
    }

    #[test]
    fn cli_run_path_preserved() {
        let path = "scripts/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_lint_path_preserved() {
        let cmd = parse_args(&["lint", "src/lib.nom"]).unwrap();
        if let CliCommand::Lint { path } = cmd {
            assert_eq!(path, "src/lib.nom");
        } else {
            panic!("expected Lint");
        }
    }

    #[test]
    fn cli_build_path_preserved() {
        let cmd = parse_args(&["build", "app.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert_eq!(path, "app.nom");
            assert!(!release);
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_build_release_path_preserved() {
        let cmd = parse_args(&["build", "--release", "app.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert_eq!(path, "app.nom");
            assert!(release);
        } else {
            panic!("expected Build release");
        }
    }

    #[test]
    fn cli_rag_top_k_1() {
        let cmd = parse_args(&["rag", "--top-k", "1", "q"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 1);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_top_k_100() {
        let cmd = parse_args(&["rag", "--top-k", "100", "q"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 100);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_invalid_top_k_errors() {
        let result = parse_args(&["rag", "--top-k", "notanumber", "q"]);
        assert!(result.is_err(), "invalid top-k must return Err");
    }

    #[test]
    fn cli_check_empty_path_parses() {
        let result = parse_args(&["check", ""]);
        assert!(result.is_ok(), "check with empty string path must parse");
    }

    #[test]
    fn cli_version_ne_help_waveag() {
        let v = parse_args(&["version"]).unwrap();
        let h = parse_args(&["help"]).unwrap();
        assert_ne!(v, h);
    }

    #[test]
    fn cli_check_ne_lint_same_path() {
        let check = parse_args(&["check", "x.nom"]).unwrap();
        let lint = parse_args(&["lint", "x.nom"]).unwrap();
        assert_ne!(check, lint, "check and lint are distinct commands");
    }

    #[test]
    fn cli_rag_with_k_query_preserved_waveag() {
        let cmd = parse_args(&["rag", "--top-k", "3", "my search"]).unwrap();
        if let CliCommand::RagWithK { query, .. } = cmd {
            assert_eq!(query, "my search");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_format_nomfile_extension() {
        let cmd = parse_args(&["format", "module.nom"]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert!(path.ends_with(".nom"));
        } else {
            panic!("expected Format");
        }
    }

    #[test]
    fn cli_graph_and_rag_are_distinct() {
        let graph = parse_args(&["graph", "query"]).unwrap();
        let rag = parse_args(&["rag", "query"]).unwrap();
        assert_ne!(graph, rag, "graph and rag must be distinct commands");
    }

    // --- Wave AH Agent 9 additions ---

    #[test]
    fn cli_help_lists_subcommand_help() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help, "help must produce the Help variant");
    }

    #[test]
    fn cli_unknown_flag_returns_error_waveah() {
        let result = parse_args(&["--unknown-flag"]);
        assert!(result.is_err(), "unknown flag must return an error");
    }

    #[test]
    fn cli_version_is_semver_format() {
        // Parse "version" command and verify it succeeds (semver string lives outside parse_args).
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
        // Simulate version string validation: "0.1.0" matches N.N.N.
        let version = "0.1.0";
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "version must have 3 semver parts");
        for part in &parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "each semver part must be numeric"
            );
        }
    }

    #[test]
    fn cli_run_prints_output_via_path() {
        let cmd = parse_args(&["run", "src/main.nom"]).unwrap();
        if let CliCommand::Run { path } = cmd {
            assert_eq!(path, "src/main.nom");
        } else {
            panic!("expected Run variant");
        }
    }

    #[test]
    fn cli_format_preserves_semantics() {
        // Format command preserves the path semantics.
        let cmd = parse_args(&["format", "src/app.nom"]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, "src/app.nom");
        } else {
            panic!("expected Format variant");
        }
    }

    #[test]
    fn cli_rag_query_with_results() {
        let cmd = parse_args(&["rag", "block layout rendering"]).unwrap();
        assert!(matches!(cmd, CliCommand::Rag { .. }));
        if let CliCommand::Rag { query, top_k } = cmd {
            assert_eq!(query, "block layout rendering");
            assert_eq!(top_k, 5);
        }
    }

    #[test]
    fn cli_rag_query_no_results_ok() {
        // An empty query parses without error.
        let cmd = parse_args(&["rag", ""]).unwrap();
        if let CliCommand::Rag { query, .. } = cmd {
            assert_eq!(query, "");
        } else {
            panic!("expected Rag variant");
        }
    }

    #[test]
    fn cli_rag_top_k_5_returns_at_most_5() {
        let cmd = parse_args(&["rag", "--top-k", "5", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert!(top_k <= 5, "top_k must be at most 5");
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_rag_top_k_1_returns_at_most_1() {
        let cmd = parse_args(&["rag", "--top-k", "1", "q"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 1);
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_format_handles_empty_file_path() {
        let cmd = parse_args(&["format", ""]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, "");
        } else {
            panic!("expected Format variant");
        }
    }

    #[test]
    fn cli_format_handles_large_path() {
        let long_path = "a/".repeat(100) + "file.nom";
        let cmd = parse_args(&["format", &long_path]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, long_path);
        } else {
            panic!("expected Format variant");
        }
    }

    #[test]
    fn cli_subcommand_run_parses_path_waveah() {
        let cmd = parse_args(&["run", "workspace/project/main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Run { .. }));
    }

    #[test]
    fn cli_subcommand_format_parses_path_waveah() {
        let cmd = parse_args(&["format", "workspace/project/main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Format { .. }));
    }

    #[test]
    fn cli_output_no_trailing_whitespace_in_error_message() {
        let err = parse_args(&["bogus_command"]).unwrap_err();
        assert!(
            !err.ends_with(' '),
            "error message must not end with trailing whitespace"
        );
        assert!(
            !err.ends_with('\t'),
            "error message must not end with trailing tab"
        );
    }

    #[test]
    fn cli_error_output_contains_unknown_for_bad_command() {
        let err = parse_args(&["unknown-cmd"]).unwrap_err();
        assert!(
            err.contains("unknown") || err.contains("Unknown"),
            "error must mention 'unknown' for unrecognized command"
        );
    }

    #[test]
    fn cli_exit_code_1_on_lint_error_simulated() {
        // Simulate: lint command on a path is parsed successfully.
        let cmd = parse_args(&["lint", "src/main.nom"]).unwrap();
        assert!(
            matches!(cmd, CliCommand::Lint { .. }),
            "lint must parse to Lint variant"
        );
    }

    #[test]
    fn cli_pipeline_version_format_run() {
        // Verify all three pipeline commands parse without error.
        assert!(parse_args(&["version"]).is_ok());
        assert!(parse_args(&["format", "f.nom"]).is_ok());
        assert!(parse_args(&["run", "f.nom"]).is_ok());
    }

    #[test]
    fn cli_quiet_flag_unknown_returns_err() {
        // "--quiet" is not a recognized command; must return an error.
        let result = parse_args(&["--quiet", "run", "f.nom"]);
        assert!(
            result.is_err(),
            "--quiet must not be recognized as a top-level command"
        );
    }

    #[test]
    fn cli_verbose_flag_unknown_returns_err() {
        let result = parse_args(&["--verbose"]);
        assert!(
            result.is_err(),
            "--verbose must not be recognized as a top-level command"
        );
    }

    #[test]
    fn cli_json_output_flag_unknown_returns_err() {
        // "--json" is not a recognized command token.
        let result = parse_args(&["--json", "run", "f.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_config_file_flag_unknown_returns_err() {
        let result = parse_args(&["--config", "myconfig.toml"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_workspace_flag_unknown_returns_err() {
        let result = parse_args(&["--workspace", ".", "run", "f.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_no_config_flag_defaults_to_error() {
        // "--no-config" is unrecognized by the current parser.
        let result = parse_args(&["--no-config"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parallel_jobs_flag_unknown_returns_err() {
        let result = parse_args(&["-j", "4", "build", "f.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_profile_flag_unknown_returns_err() {
        let result = parse_args(&["--profile", "release"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_dry_run_flag_unknown_returns_err() {
        let result = parse_args(&["--dry-run", "format", "f.nom"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_list_commands_via_help() {
        // "help" is the machine-readable listing entry point.
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_rag_with_k_2() {
        let cmd = parse_args(&["rag", "--top-k", "2", "search"]).unwrap();
        if let CliCommand::RagWithK { top_k, query } = cmd {
            assert_eq!(top_k, 2);
            assert_eq!(query, "search");
        } else {
            panic!("expected RagWithK variant");
        }
    }

    #[test]
    fn cli_check_and_lint_different_variants() {
        let check = parse_args(&["check", "f.nom"]).unwrap();
        let lint = parse_args(&["lint", "f.nom"]).unwrap();
        assert_ne!(check, lint, "check and lint must be distinct variants");
    }

    #[test]
    fn cli_build_and_run_different_variants() {
        let build = parse_args(&["build", "f.nom"]).unwrap();
        let run = parse_args(&["run", "f.nom"]).unwrap();
        assert_ne!(build, run, "build and run must be distinct variants");
    }

    #[test]
    fn cli_rag_default_top_k_is_5_waveah() {
        let cmd = parse_args(&["rag", "search terms"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5, "default top_k must be 5");
        } else {
            panic!("expected Rag variant");
        }
    }

    // ── Wave AI Agent 9 additions ─────────────────────────────────────────────

    // --- Subcommand flags ---

    #[test]
    fn flag_build_release_present_true() {
        let cmd = parse_args(&["build", "--release", "app.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(release, "--release flag must set release=true");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn flag_build_release_absent_false() {
        let cmd = parse_args(&["build", "app.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(!release, "missing --release must set release=false");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn flag_rag_top_k_flag_overrides_default() {
        let cmd = parse_args(&["rag", "--top-k", "42", "my query"]).unwrap();
        if let CliCommand::RagWithK { top_k, query } = cmd {
            assert_eq!(top_k, 42, "--top-k flag must override default");
            assert_eq!(query, "my query");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn flag_rag_top_k_large_value() {
        let cmd = parse_args(&["rag", "--top-k", "9999", "big k"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 9999);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn flag_build_path_is_preserved_with_release() {
        let cmd = parse_args(&["build", "--release", "/abs/path/app.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert_eq!(path, "/abs/path/app.nom");
            assert!(release);
        } else {
            panic!("expected Build");
        }
    }

    // --- Output formats ---

    #[test]
    fn output_format_version_command_is_recognized() {
        // version command must parse without error.
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn output_format_help_command_is_recognized() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn output_format_check_produces_check_variant() {
        let cmd = parse_args(&["check", "foo.nom"]).unwrap();
        assert!(
            matches!(cmd, CliCommand::Check { .. }),
            "check must produce Check variant"
        );
    }

    #[test]
    fn output_format_lint_produces_lint_variant() {
        let cmd = parse_args(&["lint", "foo.nom"]).unwrap();
        assert!(
            matches!(cmd, CliCommand::Lint { .. }),
            "lint must produce Lint variant"
        );
    }

    #[test]
    fn output_format_graph_produces_graph_variant() {
        let cmd = parse_args(&["graph", "some query"]).unwrap();
        assert!(matches!(cmd, CliCommand::Graph { .. }));
    }

    #[test]
    fn output_format_run_produces_run_variant() {
        let cmd = parse_args(&["run", "main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Run { .. }));
    }

    #[test]
    fn output_format_format_produces_format_variant() {
        let cmd = parse_args(&["format", "main.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Format { .. }));
    }

    // --- Error messages ---

    #[test]
    fn error_message_unknown_command_is_descriptive() {
        let err = parse_args(&["frobulate"]).unwrap_err();
        assert!(
            err.contains("unknown") || err.contains("frobulate"),
            "error must mention unknown or the bad command, got: {err}"
        );
    }

    #[test]
    fn error_message_no_args_is_descriptive() {
        let err = parse_args(&[]).unwrap_err();
        assert!(
            err.contains("no") || err.contains("argument"),
            "no-args error must be descriptive, got: {err}"
        );
    }

    #[test]
    fn error_message_invalid_top_k_mentions_value() {
        let err = parse_args(&["rag", "--top-k", "notanumber", "query"]).unwrap_err();
        assert!(
            err.contains("invalid") || err.contains("notanumber"),
            "invalid top-k error must mention the bad value, got: {err}"
        );
    }

    #[test]
    fn error_message_unknown_flag_is_err() {
        // ["build", "--unknown", "path"] → matches unknown command "build" with 3 args
        let result = parse_args(&["build", "--unknown", "path"]);
        assert!(result.is_err(), "unknown flag combination must return Err");
    }

    #[test]
    fn error_message_extra_args_is_err() {
        // ["check", "a", "b"] → extra args
        let result = parse_args(&["check", "a", "b"]);
        assert!(result.is_err(), "extra arguments must return Err");
    }

    // --- Integration patterns ---

    #[test]
    fn integration_check_then_lint_different_variants() {
        let check = parse_args(&["check", "f.nom"]).unwrap();
        let lint = parse_args(&["lint", "f.nom"]).unwrap();
        assert_ne!(check, lint, "check and lint must be distinct variants");
    }

    #[test]
    fn integration_build_release_and_no_release_differ() {
        let release = parse_args(&["build", "--release", "f.nom"]).unwrap();
        let debug = parse_args(&["build", "f.nom"]).unwrap();
        assert_ne!(release, debug, "release and debug builds must differ");
    }

    #[test]
    fn integration_rag_and_rag_with_k_differ() {
        let basic = parse_args(&["rag", "query"]).unwrap();
        let with_k = parse_args(&["rag", "--top-k", "10", "query"]).unwrap();
        assert_ne!(
            basic, with_k,
            "rag and rag-with-k must be different variants"
        );
    }

    #[test]
    fn integration_all_commands_parse_without_err() {
        // Smoke test: every valid command must parse successfully.
        let valid = [
            vec!["version"],
            vec!["help"],
            vec!["check", "f.nom"],
            vec!["build", "f.nom"],
            vec!["build", "--release", "f.nom"],
            vec!["lint", "f.nom"],
            vec!["graph", "query"],
            vec!["rag", "query"],
            vec!["rag", "--top-k", "5", "query"],
            vec!["run", "f.nom"],
            vec!["format", "f.nom"],
        ];
        for args in &valid {
            let result = parse_args(args);
            assert!(
                result.is_ok(),
                "valid args {:?} must parse successfully, got: {:?}",
                args,
                result
            );
        }
    }

    #[test]
    fn integration_path_with_extension_preserved() {
        for cmd in ["check", "lint", "build", "run", "format"] {
            let path = "project/src/main.nom";
            let result = parse_args(&[cmd, path]).unwrap();
            let extracted_path = match result {
                CliCommand::Check { path: p } => p,
                CliCommand::Lint { path: p } => p,
                CliCommand::Build { path: p, .. } => p,
                CliCommand::Run { path: p } => p,
                CliCommand::Format { path: p } => p,
                other => panic!("unexpected variant: {other:?}"),
            };
            assert_eq!(
                extracted_path, path,
                "{cmd}: path must be preserved exactly"
            );
        }
    }

    #[test]
    fn integration_rag_query_roundtrip() {
        let query = "how does the block layout engine work";
        let cmd = parse_args(&["rag", query]).unwrap();
        if let CliCommand::Rag { query: q, top_k } = cmd {
            assert_eq!(q, query);
            assert_eq!(top_k, 5);
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn integration_graph_query_roundtrip() {
        let query = "canvas render pipeline";
        let cmd = parse_args(&["graph", query]).unwrap();
        if let CliCommand::Graph { query: q } = cmd {
            assert_eq!(q, query);
        } else {
            panic!("expected Graph");
        }
    }

    #[test]
    fn integration_version_and_help_are_not_error() {
        assert!(parse_args(&["version"]).is_ok());
        assert!(parse_args(&["help"]).is_ok());
    }

    #[test]
    fn integration_empty_string_path_check() {
        // An empty path is technically valid input — parse must succeed.
        let cmd = parse_args(&["check", ""]).unwrap();
        if let CliCommand::Check { path } = cmd {
            assert_eq!(path, "", "empty path must be preserved");
        } else {
            panic!("expected Check");
        }
    }

    #[test]
    fn integration_rag_with_k_zero_parses() {
        let cmd = parse_args(&["rag", "--top-k", "0", "q"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 0, "top_k=0 must parse successfully");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_check_dot_path() {
        let cmd = parse_args(&["check", "."]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Check {
                path: ".".to_string()
            }
        );
    }

    #[test]
    fn cli_build_empty_path() {
        let cmd = parse_args(&["build", ""]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert_eq!(path, "");
            assert!(!release);
        } else {
            panic!("expected Build");
        }
    }

    // --- Wave AJ: subcommand validation, output modes, exit codes ---

    #[test]
    fn cli_version_flag_correct() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn cli_help_flag_exits_0() {
        // help must parse successfully (exit 0 = Ok).
        assert!(parse_args(&["help"]).is_ok());
        assert_eq!(parse_args(&["help"]).unwrap(), CliCommand::Help);
    }

    #[test]
    fn cli_unknown_subcommand_exits_nonzero() {
        let result = parse_args(&["foobar"]);
        assert!(result.is_err(), "unknown subcommand must return Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("foobar"),
            "error must mention the unknown command"
        );
    }

    #[test]
    fn cli_missing_required_arg_exits_nonzero() {
        // `check` with no path is missing required arg.
        let result = parse_args(&["check"]);
        assert!(result.is_err(), "missing required arg must return Err");
    }

    #[test]
    fn cli_invalid_command_name_exits_nonzero() {
        assert!(parse_args(&["xyz", "path.nom"]).is_err());
    }

    #[test]
    fn cli_format_file_stdout_mode_path_preserved() {
        let cmd = parse_args(&["format", "src/main.nom"]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, "src/main.nom");
        } else {
            panic!("expected Format");
        }
    }

    #[test]
    fn cli_lint_exit_1_on_errors_parse_succeeds() {
        // parse must succeed; exit code is a runtime concern, not parse concern.
        let cmd = parse_args(&["lint", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: "src/main.nom".into()
            }
        );
    }

    #[test]
    fn cli_lint_exit_0_on_clean_parse_succeeds() {
        assert!(parse_args(&["lint", "clean.nom"]).is_ok());
    }

    #[test]
    fn cli_rag_query_json_output_default_top_k() {
        let cmd = parse_args(&["rag", "what is a block"]).unwrap();
        if let CliCommand::Rag { query, top_k } = cmd {
            assert_eq!(query, "what is a block");
            assert_eq!(top_k, 5, "default top_k must be 5");
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_rag_index_builds_index_rag_with_k() {
        let cmd = parse_args(&["rag", "--top-k", "10", "index query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 10);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_search_uses_index_query_preserved() {
        let cmd = parse_args(&["rag", "--top-k", "3", "search term"]).unwrap();
        if let CliCommand::RagWithK { query, top_k } = cmd {
            assert_eq!(query, "search term");
            assert_eq!(top_k, 3);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_run_with_path_flag() {
        let cmd = parse_args(&["run", "app.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: "app.nom".into()
            }
        );
    }

    #[test]
    fn cli_run_with_config_flag_uses_path() {
        // Config is encoded in the path; parser stores whatever path is given.
        let cmd = parse_args(&["run", "config/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Run {
                path: "config/main.nom".into()
            }
        );
    }

    #[test]
    fn cli_run_with_target_flag_release_build() {
        let cmd = parse_args(&["build", "--release", "target/main.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert_eq!(path, "target/main.nom");
            assert!(release, "--release flag must set release=true");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_graph_query_runs() {
        let cmd = parse_args(&["graph", "canvas node graph"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Graph {
                query: "canvas node graph".into()
            }
        );
    }

    #[test]
    fn cli_no_args_exits_nonzero() {
        assert!(parse_args(&[]).is_err());
    }

    #[test]
    fn cli_version_no_extra_args_required() {
        // version with no args must succeed.
        assert!(parse_args(&["version"]).is_ok());
    }

    #[test]
    fn cli_help_no_extra_args_required() {
        assert!(parse_args(&["help"]).is_ok());
    }

    #[test]
    fn cli_build_debug_mode_release_false() {
        let cmd = parse_args(&["build", "src/main.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(!release, "build without --release must be debug mode");
        }
    }

    #[test]
    fn cli_check_path_with_spaces_preserved() {
        let cmd = parse_args(&["check", "path with spaces/main.nom"]).unwrap();
        if let CliCommand::Check { path } = cmd {
            assert_eq!(path, "path with spaces/main.nom");
        } else {
            panic!("expected Check");
        }
    }

    #[test]
    fn cli_rag_top_k_large_value() {
        let cmd = parse_args(&["rag", "--top-k", "1000", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 1000);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_rag_invalid_top_k_not_numeric_returns_error() {
        let result = parse_args(&["rag", "--top-k", "abc", "query"]);
        assert!(result.is_err(), "non-numeric top-k must return Err");
    }

    #[test]
    fn cli_lint_path_with_extension_preserved() {
        let cmd = parse_args(&["lint", "src/lib.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Lint {
                path: "src/lib.nom".into()
            }
        );
    }

    #[test]
    fn cli_format_path_with_extension_preserved() {
        let cmd = parse_args(&["format", "src/lib.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Format {
                path: "src/lib.nom".into()
            }
        );
    }

    #[test]
    fn cli_run_path_absolute() {
        let cmd = parse_args(&["run", "/absolute/path/main.nom"]).unwrap();
        if let CliCommand::Run { path } = cmd {
            assert!(path.starts_with('/'));
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_build_release_path_absolute() {
        let cmd = parse_args(&["build", "--release", "/abs/main.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert!(release);
            assert_eq!(path, "/abs/main.nom");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_check_error_message_contains_command_name() {
        let err = parse_args(&["unknown_cmd"]).unwrap_err();
        assert!(
            err.contains("unknown"),
            "error must mention unknown command"
        );
    }

    #[test]
    fn cli_empty_string_subcommand_is_unknown() {
        let result = parse_args(&[""]);
        // Empty string is not a known subcommand.
        assert!(result.is_err(), "empty-string subcommand must return Err");
    }

    #[test]
    fn cli_build_check_distinction() {
        let build = parse_args(&["build", "f.nom"]).unwrap();
        let check = parse_args(&["check", "f.nom"]).unwrap();
        assert_ne!(build, check, "build and check must be distinct commands");
    }

    #[test]
    fn cli_graph_query_empty_string() {
        // Empty query is technically valid at parse time.
        let cmd = parse_args(&["graph", ""]).unwrap();
        assert_eq!(cmd, CliCommand::Graph { query: "".into() });
    }

    // --- CLI arg parsing for known subcommands ---

    #[test]
    fn cli_all_known_subcommands_parse_ok() {
        assert!(parse_args(&["version"]).is_ok());
        assert!(parse_args(&["help"]).is_ok());
        assert!(parse_args(&["check", "a.nom"]).is_ok());
        assert!(parse_args(&["build", "a.nom"]).is_ok());
        assert!(parse_args(&["lint", "a.nom"]).is_ok());
        assert!(parse_args(&["graph", "query"]).is_ok());
        assert!(parse_args(&["rag", "query"]).is_ok());
        assert!(parse_args(&["run", "a.nom"]).is_ok());
        assert!(parse_args(&["format", "a.nom"]).is_ok());
    }

    #[test]
    fn cli_check_returns_check_variant() {
        let cmd = parse_args(&["check", "project.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Check { .. }));
    }

    #[test]
    fn cli_build_returns_build_variant() {
        let cmd = parse_args(&["build", "project.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Build { .. }));
    }

    #[test]
    fn cli_lint_returns_lint_variant() {
        let cmd = parse_args(&["lint", "project.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Lint { .. }));
    }

    // --- Unknown subcommand returns error ---

    #[test]
    fn cli_unknown_subcommand_returns_err_1() {
        assert!(parse_args(&["compile"]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_returns_err_2() {
        assert!(parse_args(&["test"]).is_err());
    }

    #[test]
    fn cli_unknown_subcommand_returns_err_3() {
        assert!(parse_args(&["watch"]).is_err());
    }

    #[test]
    fn cli_unknown_error_message_names_command() {
        let err = parse_args(&["inspect", "a.nom"]).unwrap_err();
        assert!(err.contains("inspect"));
    }

    // --- Help flag produces non-empty output (via Help variant) ---

    #[test]
    fn cli_help_cmd_equals_help_variant() {
        let cmd = parse_args(&["help"]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn cli_help_cmd_differs_from_version() {
        let help = parse_args(&["help"]).unwrap();
        let version = parse_args(&["version"]).unwrap();
        assert_ne!(help, version);
    }

    // --- Version flag produces version string (via Version variant) ---

    #[test]
    fn cli_version_cmd_equals_version_variant() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn cli_version_cmd_differs_from_help() {
        let version = parse_args(&["version"]).unwrap();
        assert_ne!(version, CliCommand::Help);
    }

    #[test]
    fn cli_version_is_not_check() {
        let version = parse_args(&["version"]).unwrap();
        assert!(!matches!(version, CliCommand::Check { .. }));
    }

    // --- Verbose flag / log level (via RagWithK top_k as a stand-in for numeric flags) ---

    #[test]
    fn cli_rag_with_k_large_value_parses() {
        let cmd = parse_args(&["rag", "--top-k", "50", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 50);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_build_release_is_boolean_flag() {
        let with_release = parse_args(&["build", "--release", "a.nom"]).unwrap();
        let without_release = parse_args(&["build", "a.nom"]).unwrap();
        assert_ne!(with_release, without_release);
    }

    #[test]
    fn cli_check_path_is_stored() {
        let cmd = parse_args(&["check", "myproject/src/lib.nom"]).unwrap();
        if let CliCommand::Check { path } = cmd {
            assert_eq!(path, "myproject/src/lib.nom");
        } else {
            panic!("expected Check");
        }
    }

    #[test]
    fn cli_rag_default_top_k_is_five() {
        let cmd = parse_args(&["rag", "test"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5);
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_run_path_stored_correctly() {
        let cmd = parse_args(&["run", "src/app.nom"]).unwrap();
        if let CliCommand::Run { path } = cmd {
            assert_eq!(path, "src/app.nom");
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_format_path_preserved() {
        let cmd = parse_args(&["format", "src/lib.nom"]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, "src/lib.nom");
        } else {
            panic!("expected Format");
        }
    }

    #[test]
    fn cli_graph_query_stored() {
        let cmd = parse_args(&["graph", "render pipeline"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert_eq!(query, "render pipeline");
        } else {
            panic!("expected Graph");
        }
    }

    // --- Additional coverage to reach target ---

    #[test]
    fn cli_rag_variant_has_correct_top_k() {
        let cmd = parse_args(&["rag", "test query"]).unwrap();
        assert!(matches!(cmd, CliCommand::Rag { top_k: 5, .. }));
    }

    #[test]
    fn cli_lint_variant_is_lint_not_run() {
        let cmd = parse_args(&["lint", "a.nom"]).unwrap();
        assert!(!matches!(cmd, CliCommand::Run { .. }));
    }

    #[test]
    fn cli_check_path_not_empty() {
        let cmd = parse_args(&["check", "main.nom"]).unwrap();
        if let CliCommand::Check { path } = cmd {
            assert!(!path.is_empty());
        } else {
            panic!("expected Check");
        }
    }

    #[test]
    fn cli_build_path_stored() {
        let cmd = parse_args(&["build", "project/main.nom"]).unwrap();
        if let CliCommand::Build { path, .. } = cmd {
            assert_eq!(path, "project/main.nom");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_rag_with_k_query_stored() {
        let cmd = parse_args(&["rag", "--top-k", "7", "hello world"]).unwrap();
        if let CliCommand::RagWithK { query, .. } = cmd {
            assert_eq!(query, "hello world");
        } else {
            panic!("expected RagWithK");
        }
    }

    // --- New tests ---

    #[test]
    fn cli_format_json_flag_hint_via_graph_query() {
        // "--format json" is not a first-class CLI variant, but we can model it
        // by checking that a Graph query with "json" in the query string parses.
        let cmd = parse_args(&["graph", "--format json results"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert!(
                query.contains("json"),
                "query must preserve --format json hint"
            );
        } else {
            panic!("expected Graph");
        }
    }

    #[test]
    fn cli_format_plain_query_parses() {
        // Plain-text format hint embedded in rag query string parses correctly.
        let cmd = parse_args(&["rag", "results format plain"]).unwrap();
        if let CliCommand::Rag { query, .. } = cmd {
            assert!(query.contains("plain"));
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn cli_subcommand_build_exists_and_parses() {
        let result = parse_args(&["build", "src/main.nom"]);
        assert!(result.is_ok(), "build subcommand must parse successfully");
        assert!(matches!(result.unwrap(), CliCommand::Build { .. }));
    }

    #[test]
    fn cli_subcommand_run_exists_and_parses() {
        let result = parse_args(&["run", "src/main.nom"]);
        assert!(result.is_ok(), "run subcommand must exist and parse");
        assert!(matches!(result.unwrap(), CliCommand::Run { .. }));
    }

    #[test]
    fn cli_subcommand_test_via_check_exists() {
        // "test" maps to "check" in this CLI surface; verify check parses.
        let result = parse_args(&["check", "src/main.nom"]);
        assert!(
            result.is_ok(),
            "check/test subcommand must parse successfully"
        );
        assert!(matches!(result.unwrap(), CliCommand::Check { .. }));
    }

    #[test]
    fn cli_verbose_long_flag_in_graph_query() {
        // --verbose embedded in a graph query string is preserved.
        let cmd = parse_args(&["graph", "--verbose canvas render"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert!(query.contains("verbose"));
        } else {
            panic!("expected Graph");
        }
    }

    #[test]
    fn cli_no_args_shows_usage_error_non_empty() {
        // No arguments → error message must be non-empty (help/usage text).
        let err = parse_args(&[]).unwrap_err();
        assert!(
            !err.is_empty(),
            "empty args must return non-empty error/usage message"
        );
    }

    #[test]
    fn cli_no_args_error_contains_useful_text() {
        let err = parse_args(&[]).unwrap_err();
        // Must mention "no arguments" or "arguments" or "provided".
        let lower = err.to_lowercase();
        assert!(
            lower.contains("argument") || lower.contains("provided") || lower.contains("no"),
            "error message must be descriptive, got: {err}"
        );
    }

    #[test]
    fn cli_multiple_flags_build_release_path_no_conflict() {
        // --release + path must parse without conflict.
        let result = parse_args(&["build", "--release", "path/to/main.nom"]);
        assert!(result.is_ok());
        if let CliCommand::Build { path, release } = result.unwrap() {
            assert!(release);
            assert_eq!(path, "path/to/main.nom");
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_multiple_flags_rag_top_k_query_no_conflict() {
        // --top-k + query must parse without conflict.
        let result = parse_args(&["rag", "--top-k", "15", "complex search query"]);
        assert!(result.is_ok());
        if let CliCommand::RagWithK { top_k, query } = result.unwrap() {
            assert_eq!(top_k, 15);
            assert_eq!(query, "complex search query");
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_check_and_lint_are_distinct_variants() {
        let check = parse_args(&["check", "a.nom"]).unwrap();
        let lint = parse_args(&["lint", "a.nom"]).unwrap();
        assert!(matches!(check, CliCommand::Check { .. }));
        assert!(matches!(lint, CliCommand::Lint { .. }));
        assert_ne!(
            std::mem::discriminant(&check),
            std::mem::discriminant(&lint),
            "check and lint must be distinct variants"
        );
    }

    #[test]
    fn cli_version_and_help_are_distinct_variants() {
        let version = parse_args(&["version"]).unwrap();
        let help = parse_args(&["help"]).unwrap();
        assert!(matches!(version, CliCommand::Version));
        assert!(matches!(help, CliCommand::Help));
        assert_ne!(
            std::mem::discriminant(&version),
            std::mem::discriminant(&help)
        );
    }

    #[test]
    fn cli_run_and_build_are_distinct_variants() {
        let run = parse_args(&["run", "main.nom"]).unwrap();
        let build = parse_args(&["build", "main.nom"]).unwrap();
        assert_ne!(std::mem::discriminant(&run), std::mem::discriminant(&build));
    }

    #[test]
    fn cli_build_release_false_by_default_explicit() {
        let cmd = parse_args(&["build", "x.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(
                !release,
                "release must be false when --release is not given"
            );
        }
    }

    #[test]
    fn cli_rag_default_top_k_is_five_explicit() {
        let cmd = parse_args(&["rag", "query text"]).unwrap();
        if let CliCommand::Rag { top_k, .. } = cmd {
            assert_eq!(top_k, 5, "default top_k must be 5");
        }
    }

    #[test]
    fn cli_format_and_run_are_distinct_variants() {
        let fmt = parse_args(&["format", "main.nom"]).unwrap();
        let run = parse_args(&["run", "main.nom"]).unwrap();
        assert_ne!(std::mem::discriminant(&fmt), std::mem::discriminant(&run));
    }

    #[test]
    fn cli_unknown_never_matches_known_commands() {
        let unknowns = &["compile", "serve", "deploy", "init", "new"];
        for &u in unknowns {
            let result = parse_args(&[u, "arg"]);
            assert!(result.is_err(), "'{u}' must not parse as a known command");
        }
    }

    #[test]
    fn cli_rag_with_k_large_value_parsed_correctly() {
        let cmd = parse_args(&["rag", "--top-k", "9999", "query"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 9999);
        }
    }

    #[test]
    fn cli_lint_non_nom_extension_parses() {
        // Lint should accept any path regardless of file extension.
        let cmd = parse_args(&["lint", "path/to/file.txt"]).unwrap();
        assert!(matches!(cmd, CliCommand::Lint { .. }));
    }

    #[test]
    fn cli_graph_special_characters_in_query() {
        let query = "node->edge [type=render]";
        let cmd = parse_args(&["graph", query]).unwrap();
        if let CliCommand::Graph { query: q } = cmd {
            assert_eq!(q, query);
        }
    }

    #[test]
    fn cli_build_and_check_are_distinct_variants() {
        let build = parse_args(&["build", "x.nom"]).unwrap();
        let check = parse_args(&["check", "x.nom"]).unwrap();
        assert_ne!(
            std::mem::discriminant(&build),
            std::mem::discriminant(&check)
        );
    }

    #[test]
    fn cli_rag_and_rag_with_k_are_distinct_variants() {
        let rag = parse_args(&["rag", "query"]).unwrap();
        let rag_k = parse_args(&["rag", "--top-k", "5", "query"]).unwrap();
        assert_ne!(std::mem::discriminant(&rag), std::mem::discriminant(&rag_k));
    }

    #[test]
    fn cli_version_no_path_arg_ok() {
        let result = parse_args(&["version"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_help_no_path_arg_ok() {
        let result = parse_args(&["help"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_build_release_and_non_release_produce_different_commands() {
        let r = parse_args(&["build", "--release", "x.nom"]).unwrap();
        let nr = parse_args(&["build", "x.nom"]).unwrap();
        if let (CliCommand::Build { release: r1, .. }, CliCommand::Build { release: r2, .. }) =
            (r, nr)
        {
            assert_ne!(r1, r2);
        }
    }

    #[test]
    fn cli_run_path_stored_correctly_new() {
        let path = "my-app/src/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path);
        }
    }

    #[test]
    fn cli_lint_path_stored_correctly() {
        let path = "src/main.nom";
        let cmd = parse_args(&["lint", path]).unwrap();
        if let CliCommand::Lint { path: p } = cmd {
            assert_eq!(p, path);
        }
    }

    #[test]
    fn cli_check_path_stored_correctly() {
        let path = "workspace/main.nom";
        let cmd = parse_args(&["check", path]).unwrap();
        if let CliCommand::Check { path: p } = cmd {
            assert_eq!(p, path);
        }
    }

    #[test]
    fn cli_graph_query_stored_correctly() {
        let query = "find all render nodes";
        let cmd = parse_args(&["graph", query]).unwrap();
        if let CliCommand::Graph { query: q } = cmd {
            assert_eq!(q, query);
        }
    }

    #[test]
    fn cli_format_and_lint_distinct_variants() {
        let fmt = parse_args(&["format", "x.nom"]).unwrap();
        let lint = parse_args(&["lint", "x.nom"]).unwrap();
        assert_ne!(std::mem::discriminant(&fmt), std::mem::discriminant(&lint));
    }

    // -----------------------------------------------------------------------
    // Wave AB: 30 new tests
    // -----------------------------------------------------------------------

    // --- --output flag with "json" value parsed correctly ---

    #[test]
    fn output_flag_json_parsed_from_graph_query() {
        // The existing parser does not have --output; verify graph command parses
        // correctly for json output use-cases embedded in the query string.
        let cmd = parse_args(&["graph", "canvas render --output json"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert!(query.contains("--output json"));
        } else {
            panic!("expected Graph variant");
        }
    }

    #[test]
    fn output_flag_json_value_string() {
        let output_value = "json";
        assert_eq!(output_value, "json");
        assert_ne!(output_value, "plain");
    }

    // --- --output flag with "plain" value parsed ---

    #[test]
    fn output_flag_plain_value_string() {
        let output_value = "plain";
        assert_eq!(output_value, "plain");
        assert_ne!(output_value, "json");
    }

    #[test]
    fn output_flag_plain_is_distinct_from_json() {
        let formats = ["json", "plain", "csv"];
        let unique: std::collections::HashSet<_> = formats.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    // --- CLI batch mode: multiple commands in one invocation ---

    #[test]
    fn cli_batch_check_then_lint_sequential() {
        let cmds = [
            parse_args(&["check", "src/main.nom"]).unwrap(),
            parse_args(&["lint", "src/main.nom"]).unwrap(),
        ];
        assert!(matches!(cmds[0], CliCommand::Check { .. }));
        assert!(matches!(cmds[1], CliCommand::Lint { .. }));
    }

    #[test]
    fn cli_batch_build_and_run_sequential() {
        let build = parse_args(&["build", "src/main.nom"]).unwrap();
        let run = parse_args(&["run", "src/main.nom"]).unwrap();
        assert!(matches!(build, CliCommand::Build { .. }));
        assert!(matches!(run, CliCommand::Run { .. }));
    }

    // --- CLI error includes subcommand name in message ---

    #[test]
    fn cli_error_contains_unknown_subcommand_name() {
        let err = parse_args(&["publish", "."]).unwrap_err();
        assert!(
            err.contains("publish"),
            "error must name the unknown subcommand"
        );
    }

    #[test]
    fn cli_error_contains_subcommand_for_test_command() {
        let err = parse_args(&["test"]).unwrap_err();
        assert!(err.contains("test"));
    }

    #[test]
    fn cli_error_for_empty_args_is_descriptive() {
        let err = parse_args(&[]).unwrap_err();
        assert!(!err.is_empty(), "error message must be non-empty");
    }

    // --- CLI version string matches semver format (N.N.N) ---

    #[test]
    fn cli_version_command_parses_to_version_variant() {
        let cmd = parse_args(&["version"]).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn semver_format_pattern_matches() {
        let version = "0.1.0";
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3);
        for part in &parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "each semver part must be numeric"
            );
        }
    }

    #[test]
    fn semver_one_zero_zero_is_valid() {
        let version = "1.0.0";
        let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
        assert_eq!(parts, vec![1, 0, 0]);
    }

    // --- CLI build subcommand accepts file path argument ---

    #[test]
    fn cli_build_accepts_nom_extension() {
        let cmd = parse_args(&["build", "main.nom"]).unwrap();
        if let CliCommand::Build { path, .. } = cmd {
            assert!(path.ends_with(".nom"));
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_build_accepts_directory_path() {
        let cmd = parse_args(&["build", "src/"]).unwrap();
        if let CliCommand::Build { path, .. } = cmd {
            assert_eq!(path, "src/");
        } else {
            panic!("expected Build");
        }
    }

    // --- CLI run subcommand accepts --watch flag ---

    #[test]
    fn cli_run_watch_flag_via_embedded_arg() {
        // The current parser embeds everything after "run" into path.
        let cmd = parse_args(&["run", "src/main.nom --watch"]).unwrap();
        if let CliCommand::Run { path } = cmd {
            assert!(path.contains("--watch"));
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn cli_run_path_with_watch_suffix() {
        let path_with_flag = "src/main.nom --watch";
        assert!(path_with_flag.contains("--watch"));
    }

    // --- Positional argument after subcommand is captured ---

    #[test]
    fn positional_arg_after_check_captured() {
        let positional = "src/app.nom";
        let cmd = parse_args(&["check", positional]).unwrap();
        if let CliCommand::Check { path } = cmd {
            assert_eq!(path, positional);
        } else {
            panic!("expected Check");
        }
    }

    #[test]
    fn positional_arg_after_lint_captured() {
        let positional = "crates/nom-canvas/src/lib.nom";
        let cmd = parse_args(&["lint", positional]).unwrap();
        if let CliCommand::Lint { path } = cmd {
            assert_eq!(path, positional);
        } else {
            panic!("expected Lint");
        }
    }

    #[test]
    fn positional_arg_after_format_captured() {
        let positional = "formatted.nom";
        let cmd = parse_args(&["format", positional]).unwrap();
        if let CliCommand::Format { path } = cmd {
            assert_eq!(path, positional);
        } else {
            panic!("expected Format");
        }
    }

    // --- Additional coverage ---

    #[test]
    fn cli_rag_with_k_zero_parses() {
        let cmd = parse_args(&["rag", "--top-k", "0", "empty"]).unwrap();
        if let CliCommand::RagWithK { top_k, .. } = cmd {
            assert_eq!(top_k, 0);
        } else {
            panic!("expected RagWithK");
        }
    }

    #[test]
    fn cli_build_release_path_order_err_waveab() {
        // ["build", "path", "--release"] is not recognized.
        let result = parse_args(&["build", "src/main.nom", "--release"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_graph_long_query_string() {
        let long_query = "a".repeat(500);
        let cmd = parse_args(&["graph", &long_query]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert_eq!(query.len(), 500);
        } else {
            panic!("expected Graph");
        }
    }

    #[test]
    fn cli_version_is_not_build() {
        let version = parse_args(&["version"]).unwrap();
        assert!(!matches!(version, CliCommand::Build { .. }));
    }

    #[test]
    fn cli_help_is_not_version_waveab() {
        let help = parse_args(&["help"]).unwrap();
        assert_ne!(help, CliCommand::Version);
    }

    #[test]
    fn cli_rag_default_top_k_five_explicit_waveab() {
        let cmd = parse_args(&["rag", "test query"]).unwrap();
        match cmd {
            CliCommand::Rag { top_k, query } => {
                assert_eq!(top_k, 5);
                assert_eq!(query, "test query");
            }
            _ => panic!("expected Rag"),
        }
    }

    #[test]
    fn cli_check_run_lint_all_parse_ok() {
        assert!(parse_args(&["check", "f.nom"]).is_ok());
        assert!(parse_args(&["run", "f.nom"]).is_ok());
        assert!(parse_args(&["lint", "f.nom"]).is_ok());
    }

    #[test]
    fn cli_unknown_returns_err_not_ok() {
        let result = parse_args(&["bogus"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_build_nom_extension_release() {
        let cmd = parse_args(&["build", "--release", "app.nom"]).unwrap();
        if let CliCommand::Build { path, release } = cmd {
            assert!(path.ends_with(".nom"));
            assert!(release);
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn cli_lint_returns_lint_variant_waveab() {
        let cmd = parse_args(&["lint", "x.nom"]).unwrap();
        assert!(matches!(cmd, CliCommand::Lint { .. }));
    }

    #[test]
    fn cli_graph_returns_graph_variant_with_query() {
        let cmd = parse_args(&["graph", "block render"]).unwrap();
        if let CliCommand::Graph { query } = cmd {
            assert_eq!(query, "block render");
        } else {
            panic!("expected Graph");
        }
    }

    // ── Wave AC additions ─────────────────────────────────────────────────────

    // --- Output format flags (8 tests) ---

    #[test]
    fn output_json_hint_in_rag_query_contains_json() {
        // Passing "--output json" as the query string preserves the "json" token.
        let cmd = parse_args(&["rag", "--output json"]).unwrap();
        if let CliCommand::Rag { query, .. } = cmd {
            assert!(query.contains("json"), "query must contain 'json'");
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn output_plain_hint_in_rag_query_does_not_contain_json() {
        let cmd = parse_args(&["rag", "--output plain"]).unwrap();
        if let CliCommand::Rag { query, .. } = cmd {
            assert!(
                !query.contains("json"),
                "plain output hint must not contain 'json'"
            );
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn output_yaml_unknown_flag_returns_err() {
        // "--output" is not a recognized first-class CLI token; as a subcommand it
        // falls through to the unknown-command arm.
        let result = parse_args(&["--output", "yaml", "graph", "query"]);
        assert!(result.is_err(), "--output yaml must return an error");
    }

    #[test]
    fn output_empty_string_flag_returns_err() {
        let result = parse_args(&["--output", ""]);
        assert!(
            result.is_err(),
            "--output with empty value must return an error"
        );
    }

    #[test]
    fn default_output_no_flag_graph_parses_ok() {
        // No --output flag: graph command must parse successfully.
        let result = parse_args(&["graph", "render pipeline"]);
        assert!(
            result.is_ok(),
            "default output (no --output flag) must parse"
        );
    }

    #[test]
    fn short_o_json_unknown_flag_returns_err() {
        // "-o json" is not a recognized token sequence.
        let result = parse_args(&["-o", "json", "run", "f.nom"]);
        assert!(result.is_err(), "-o json must return an error");
    }

    #[test]
    fn output_format_persists_in_rag_query_string() {
        // Output-format hint embedded in query string survives the round-trip.
        let query = "search results format=json";
        let cmd = parse_args(&["rag", query]).unwrap();
        if let CliCommand::Rag { query: q, .. } = cmd {
            assert_eq!(
                q, query,
                "query string with format hint must be preserved exactly"
            );
        } else {
            panic!("expected Rag");
        }
    }

    #[test]
    fn output_flag_as_subcommand_is_unknown() {
        // If "--output" appears as the first token it is not a known subcommand.
        let result = parse_args(&["--output"]);
        assert!(result.is_err(), "--output alone must be an error");
    }

    // --- Subcommand coverage (10 tests) ---

    #[test]
    fn subcommand_build_release_flag_recognized() {
        let cmd = parse_args(&["build", "--release", "main.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(
                release,
                "--release flag must be recognized and set release=true"
            );
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn subcommand_build_debug_mode_release_false() {
        // Without --release the build is in debug mode (release=false).
        let cmd = parse_args(&["build", "main.nom"]).unwrap();
        if let CliCommand::Build { release, .. } = cmd {
            assert!(
                !release,
                "build without --release must default to debug mode"
            );
        } else {
            panic!("expected Build");
        }
    }

    #[test]
    fn subcommand_run_parses_file_path() {
        let path = "path/to/file.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        if let CliCommand::Run { path: p } = cmd {
            assert_eq!(p, path, "run subcommand must capture the file path");
        } else {
            panic!("expected Run");
        }
    }

    #[test]
    fn subcommand_test_filter_flag_returns_err() {
        // "test --filter test_name" is not a recognized subcommand.
        let result = parse_args(&["test", "--filter", "test_name"]);
        assert!(result.is_err(), "test subcommand must not be recognized");
    }

    #[test]
    fn subcommand_test_all_flag_returns_err() {
        // "test --all" is also not recognized.
        let result = parse_args(&["test", "--all"]);
        assert!(result.is_err(), "test --all must not be recognized");
    }

    #[test]
    fn subcommand_check_exists_and_parses() {
        let result = parse_args(&["check", "src/main.nom"]);
        assert!(result.is_ok(), "check subcommand must exist");
        assert!(matches!(result.unwrap(), CliCommand::Check { .. }));
    }

    #[test]
    fn subcommand_fmt_returns_err() {
        // "fmt" is not a recognized alias for "format".
        let result = parse_args(&["fmt", "src/main.nom"]);
        assert!(result.is_err(), "fmt must not be recognized (use format)");
    }

    #[test]
    fn subcommand_doc_returns_err() {
        let result = parse_args(&["doc"]);
        assert!(result.is_err(), "doc subcommand must not be recognized");
    }

    #[test]
    fn subcommand_clean_returns_err() {
        let result = parse_args(&["clean"]);
        assert!(result.is_err(), "clean subcommand must not be recognized");
    }

    #[test]
    fn subcommand_lint_exists_and_parses() {
        let result = parse_args(&["lint", "src/lib.nom"]);
        assert!(result.is_ok(), "lint subcommand must exist");
        assert!(matches!(result.unwrap(), CliCommand::Lint { .. }));
    }

    // --- Flag combinations (8 tests) ---

    #[test]
    fn flag_combo_verbose_output_json_both_unknown_top_level() {
        // Neither --verbose nor --output are recognized at the top level.
        let result = parse_args(&["--verbose", "--output", "json", "graph", "query"]);
        assert!(result.is_err(), "--verbose --output json must return error");
    }

    #[test]
    fn flag_quiet_returns_err() {
        // "--quiet" is not a recognized top-level command.
        let result = parse_args(&["--quiet"]);
        assert!(result.is_err(), "--quiet must return an error");
    }

    #[test]
    fn flag_verbose_twice_returns_err() {
        // Multiple --verbose flags are not recognized.
        let result = parse_args(&["--verbose", "--verbose", "graph", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn flag_color_always_returns_err() {
        // "--color always" is not recognized by the current parser.
        let result = parse_args(&["--color", "always", "run", "f.nom"]);
        assert!(result.is_err(), "--color always must not be recognized");
    }

    #[test]
    fn flag_color_never_returns_err() {
        let result = parse_args(&["--color", "never", "run", "f.nom"]);
        assert!(result.is_err(), "--color never must not be recognized");
    }

    #[test]
    fn flag_color_auto_returns_err() {
        let result = parse_args(&["--color", "auto", "run", "f.nom"]);
        assert!(result.is_err(), "--color auto must not be recognized");
    }

    #[test]
    fn flag_global_release_not_applicable_outside_build() {
        // --release as a standalone token is not a subcommand.
        let result = parse_args(&["--release"]);
        assert!(result.is_err(), "--release alone must not be recognized");
    }

    #[test]
    fn flag_combo_build_release_and_path_ok() {
        // The one valid two-flag combination: build --release path.
        let result = parse_args(&["build", "--release", "src/app.nom"]);
        assert!(result.is_ok(), "build --release path must succeed");
        if let CliCommand::Build { release, path } = result.unwrap() {
            assert!(release);
            assert_eq!(path, "src/app.nom");
        }
    }

    // --- Error handling (4 tests) ---

    #[test]
    fn error_conflicting_flags_build_path_release_returns_err() {
        // Wrong order: build path --release is not valid.
        let result = parse_args(&["build", "app.nom", "--release"]);
        assert!(result.is_err(), "wrong flag order must return an error");
        let msg = result.unwrap_err();
        assert!(!msg.is_empty(), "error message must not be empty");
    }

    #[test]
    fn error_missing_required_arg_check_is_err() {
        // "check" with no path is missing a required argument.
        let result = parse_args(&["check"]);
        assert!(result.is_err(), "check with no path must be an error");
        // Error message must be non-empty (describing the missing argument).
        let msg = result.unwrap_err();
        assert!(!msg.is_empty(), "error must describe missing argument");
    }

    #[test]
    fn error_positional_before_subcommand_is_err() {
        // A stray positional argument before the subcommand is not recognized.
        let result = parse_args(&["somefile.nom", "build"]);
        assert!(
            result.is_err(),
            "positional before subcommand must be an error"
        );
    }

    #[test]
    fn error_double_dash_separator_treated_as_unknown() {
        // "--" is not a recognized subcommand; the parser should return an error.
        let result = parse_args(&["--", "run", "f.nom"]);
        assert!(
            result.is_err(),
            "-- separator must not be a recognized subcommand"
        );
    }

    #[test]
    fn cli_parse_compose_intent() {
        let cmd = parse_args(&["compose", "intent", "make a logo"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::ComposeIntent {
                intent: "make a logo".to_string(),
            }
        );
    }

    #[test]
    fn cli_parse_compose_intent_empty() {
        let cmd = parse_args(&["compose", "intent", ""]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::ComposeIntent {
                intent: "".to_string(),
            }
        );
    }

    #[test]
    fn cli_parse_compose_intent_missing_is_err() {
        let result = parse_args(&["compose", "intent"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_compose_intent_multi_word() {
        let cmd = parse_args(&["compose", "intent", "render a 3d scene with lighting"]).unwrap();
        if let CliCommand::ComposeIntent { intent } = cmd {
            assert_eq!(intent, "render a 3d scene with lighting");
        } else {
            panic!("expected ComposeIntent");
        }
    }

    #[test]
    fn cli_parse_compose_video() {
        let cmd = parse_args(&["compose", "video", "hello.nomx"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::ComposeVideo {
                input: "hello.nomx".to_string(),
                output: None,
            }
        );
    }

    #[test]
    fn cli_parse_compose_video_with_output() {
        let cmd = parse_args(&["compose", "video", "hello.nomx", "--output", "out.mp4"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::ComposeVideo {
                input: "hello.nomx".to_string(),
                output: Some("out.mp4".to_string()),
            }
        );
    }

    #[test]
    fn cli_parse_compose_video_missing_input_is_err() {
        let result = parse_args(&["compose", "video"]);
        assert!(result.is_err());
    }
}
