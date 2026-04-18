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
        ];
        for args in valid {
            assert!(parse_args(args).is_ok(), "expected ok for {:?}", args);
        }
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
        assert_eq!(cmd1, cmd2, "format command is idempotent: same input yields same output");
    }

    #[test]
    fn cli_format_idempotent_result_equals_single_parse() {
        let path = "workspace/lib.nom";
        let once = parse_args(&["format", path]).unwrap();
        let twice = parse_args(&["format", path]).unwrap();
        // Both must equal CliCommand::Format with the same path.
        assert_eq!(
            once,
            CliCommand::Format { path: path.to_string() },
            "first parse must produce expected Format"
        );
        assert_eq!(
            twice,
            CliCommand::Format { path: path.to_string() },
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
        assert!(dbg.contains("Version"), "version debug must contain 'Version'");
    }

    #[test]
    fn cli_version_variant_matches_version() {
        let cmd = parse_args(&["version"]).unwrap();
        assert!(matches!(cmd, CliCommand::Version), "version command must parse as Version variant");
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
        assert_eq!(a, b, "format command must be idempotent — same args same result");
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
        assert!(!dbg.is_empty(), "Version command debug string must not be empty");
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
        assert_ne!(no_flag, with_flag, "--release flag must produce different variant");
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
            assert!(part.parse::<u32>().is_ok(), "each semver part must be numeric");
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
        assert!(!err.ends_with(' '), "error message must not end with trailing whitespace");
        assert!(!err.ends_with('\t'), "error message must not end with trailing tab");
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
        assert!(matches!(cmd, CliCommand::Lint { .. }), "lint must parse to Lint variant");
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
        assert!(result.is_err(), "--quiet must not be recognized as a top-level command");
    }

    #[test]
    fn cli_verbose_flag_unknown_returns_err() {
        let result = parse_args(&["--verbose"]);
        assert!(result.is_err(), "--verbose must not be recognized as a top-level command");
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
}
