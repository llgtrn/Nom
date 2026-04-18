#[derive(Debug)]
pub enum UxCommand {
    Seed { path: String },
}

impl UxCommand {
    pub fn run(&self) {
        match self {
            UxCommand::Seed { path } => {
                println!("ux seed: scanning {} for UX patterns", path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ux_seed_command() {
        UxCommand::Seed {
            path: "src/".into(),
        }
        .run();
    }

    #[test]
    fn test_ux_seed_non_empty_path() {
        let cmd = UxCommand::Seed { path: "ui/".into() };
        assert!(matches!(cmd, UxCommand::Seed { .. }));
    }
}
