#[derive(Debug)]
pub enum AppCommand {
    New { name: String },
    Import { path: String },
    Build { name: String },
    BuildReport { name: String },
    ExplainSelection { selection: String },
}

impl AppCommand {
    pub fn run(&self) {
        match self {
            AppCommand::New { name } => println!("app new: creating {}", name),
            AppCommand::Import { path } => println!("app import: importing {}", path),
            AppCommand::Build { name } => println!("app build: building {}", name),
            AppCommand::BuildReport { name } => println!("app build-report: {}", name),
            AppCommand::ExplainSelection { selection } => println!("app explain: {}", selection),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        AppCommand::New { name: "myapp".into() }.run();
    }

    #[test]
    fn test_app_build() {
        AppCommand::Build { name: "myapp".into() }.run();
    }

    #[test]
    fn test_app_explain_selection() {
        AppCommand::ExplainSelection {
            selection: "fn foo() {}".into(),
        }
        .run();
    }
}
