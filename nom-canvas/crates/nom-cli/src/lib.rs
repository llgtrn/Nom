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
}
