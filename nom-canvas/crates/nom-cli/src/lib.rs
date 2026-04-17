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
        assert_eq!(cmd, CliCommand::Format { path: path.to_string() });
    }

    #[test]
    fn cli_format_relative_path_with_dots() {
        let path = "../../other/main.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(cmd, CliCommand::Format { path: path.to_string() });
    }

    #[test]
    fn cli_format_windows_style_path() {
        let path = "C:\\Users\\dev\\project\\main.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(cmd, CliCommand::Format { path: path.to_string() });
    }

    #[test]
    fn cli_format_empty_path() {
        let cmd = parse_args(&["format", ""]).unwrap();
        assert_eq!(cmd, CliCommand::Format { path: "".to_string() });
    }

    // --- Run command path formats ---

    #[test]
    fn cli_run_dot_path() {
        let cmd = parse_args(&["run", "."]).unwrap();
        assert_eq!(cmd, CliCommand::Run { path: ".".to_string() });
    }

    #[test]
    fn cli_run_windows_path() {
        let path = "C:\\project\\src\\main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(cmd, CliCommand::Run { path: path.to_string() });
    }

    #[test]
    fn cli_run_path_with_spaces() {
        let path = "my project/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(cmd, CliCommand::Run { path: path.to_string() });
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
}
