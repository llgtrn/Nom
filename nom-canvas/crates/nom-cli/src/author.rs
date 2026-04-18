use std::path::PathBuf;

pub enum AuthorCommand {
    Start { name: String, output: PathBuf },
    Check { path: PathBuf },
}

pub fn run(cmd: AuthorCommand) -> Result<(), String> {
    match cmd {
        AuthorCommand::Start { name, output } => {
            let content = format!(
                "# {name}\n\n<!-- Authoring mode: brainstorm → .nomx -->\n\n## Intent\n\n## Composition\n"
            );
            std::fs::write(&output, content).map_err(|e| e.to_string())?;
            println!("Created authoring workspace: {}", output.display());
            Ok(())
        }
        AuthorCommand::Check { path } => {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let nom_lines = content
                .lines()
                .filter(|l| !l.starts_with('#') && !l.starts_with("<!--") && !l.trim().is_empty())
                .count();
            let total_lines = content.lines().count();
            let pct = if total_lines > 0 {
                nom_lines * 100 / total_lines
            } else {
                0
            };
            println!(
                "Authoring progress: {}% Nom syntax ({}/{} lines)",
                pct, nom_lines, total_lines
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_start_creates_file() {
        let tmp = std::env::temp_dir().join("nom_author_test.md");
        run(AuthorCommand::Start {
            name: "test-app".to_string(),
            output: tmp.clone(),
        })
        .unwrap();
        assert!(tmp.exists());
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_author_check_counts_lines() {
        let tmp = std::env::temp_dir().join("nom_author_check_test.md");
        let content = "# Title\n\n<!-- comment -->\n\nsome nom line\nanother line\n";
        std::fs::write(&tmp, content).unwrap();
        run(AuthorCommand::Check { path: tmp.clone() }).unwrap();
        std::fs::remove_file(tmp).ok();
    }
}
