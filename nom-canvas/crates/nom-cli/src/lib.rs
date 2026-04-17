/// Commands understood by the nom-canvas CLI.
#[derive(Debug, PartialEq)]
pub enum CliCommand {
    Check { path: String },
    Build { path: String, release: bool },
    Lint { path: String },
    Graph { query: String },
    Rag { query: String, top_k: usize },
}

/// Parse a slice of string arguments into a [`CliCommand`].
///
/// # Errors
/// Returns `Err` when the command name is unknown or required arguments are
/// missing.
pub fn parse_args(args: &[&str]) -> Result<CliCommand, String> {
    match args {
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
}
