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
        ["run", path] => Ok(CliCommand::Run { path: path.to_string() }),
        ["format", path] => Ok(CliCommand::Format { path: path.to_string() }),
        ["rag", "--top-k", k_str, query] => {
            let top_k = k_str
                .parse::<usize>()
                .map_err(|_| format!("invalid top-k value: {}", k_str))?;
            Ok(CliCommand::RagWithK { query: query.to_string(), top_k })
        }
        ["check", path] => Ok(CliCommand::Check { path: path.to_string() }),
        ["build", "--release", path] => Ok(CliCommand::Build {
            path: path.to_string(),
            release: true,
        }),
        ["build", path] => Ok(CliCommand::Build {
            path: path.to_string(),
            release: false,
        }),
        ["lint", path] => Ok(CliCommand::Lint { path: path.to_string() }),
        ["graph", query] => Ok(CliCommand::Graph { query: query.to_string() }),
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
        assert_eq!(cmd, CliCommand::Check { path: "src/main.nom".to_string() });
    }

    #[test]
    fn cli_parse_build_default() {
        let cmd = parse_args(&["build", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build { path: "src/main.nom".to_string(), release: false }
        );
    }

    #[test]
    fn cli_parse_build_release() {
        let cmd = parse_args(&["build", "--release", "src/main.nom"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Build { path: "src/main.nom".to_string(), release: true }
        );
    }

    #[test]
    fn cli_parse_lint() {
        let cmd = parse_args(&["lint", "src/main.nom"]).unwrap();
        assert_eq!(cmd, CliCommand::Lint { path: "src/main.nom".to_string() });
    }

    #[test]
    fn cli_parse_graph() {
        let cmd = parse_args(&["graph", "canvas render"]).unwrap();
        assert_eq!(cmd, CliCommand::Graph { query: "canvas render".to_string() });
    }

    #[test]
    fn cli_parse_rag() {
        let cmd = parse_args(&["rag", "block layout"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Rag { query: "block layout".to_string(), top_k: 5 }
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
        assert_eq!(cmd, CliCommand::Check { path: path.to_string() });
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
        assert_eq!(cmd, CliCommand::Lint { path: "crates/nom-core".to_string() });
    }

    #[test]
    fn cli_parse_graph_query_preserved() {
        let cmd = parse_args(&["graph", "node edge type"]).unwrap();
        assert_eq!(cmd, CliCommand::Graph { query: "node edge type".to_string() });
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
            CliCommand::Rag { query: "canvas layout algorithm".to_string(), top_k: 5 }
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
        assert_eq!(cmd, CliCommand::Run { path: "src/main.nom".to_string() });
    }

    #[test]
    fn cli_parse_format() {
        let cmd = parse_args(&["format", "src/main.nom"]).unwrap();
        assert_eq!(cmd, CliCommand::Format { path: "src/main.nom".to_string() });
    }

    #[test]
    fn cli_parse_rag_with_top_k() {
        let cmd = parse_args(&["rag", "--top-k", "10", "canvas layout"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::RagWithK { query: "canvas layout".to_string(), top_k: 10 }
        );
    }

    #[test]
    fn cli_parse_rag_top_k_five() {
        let cmd = parse_args(&["rag", "--top-k", "5", "query"]).unwrap();
        assert_eq!(cmd, CliCommand::RagWithK { query: "query".to_string(), top_k: 5 });
    }

    #[test]
    fn cli_parse_rag_top_k_twenty() {
        let cmd = parse_args(&["rag", "--top-k", "20", "search term"]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::RagWithK { query: "search term".to_string(), top_k: 20 }
        );
    }

    #[test]
    fn cli_parse_run_absolute_path() {
        let path = "/home/user/project/src/main.nom";
        let cmd = parse_args(&["run", path]).unwrap();
        assert_eq!(cmd, CliCommand::Run { path: path.to_string() });
    }

    #[test]
    fn cli_parse_format_path_preserved() {
        let path = "crates/nom-canvas-core/src/lib.nom";
        let cmd = parse_args(&["format", path]).unwrap();
        assert_eq!(cmd, CliCommand::Format { path: path.to_string() });
    }

    #[test]
    fn cli_parse_rag_invalid_top_k_returns_err() {
        let result = parse_args(&["rag", "--top-k", "abc", "query"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid"));
    }
}
