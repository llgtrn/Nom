pub enum FlowCommand {
    Record { name: String },
    Show { artifact_id: String },
    Diff { id_a: String, id_b: String },
    Middleware { artifact_id: String },
}

pub fn run(cmd: FlowCommand) -> Result<(), String> {
    match cmd {
        FlowCommand::Record { name } => {
            println!("Recording flow: {}", name);
            Ok(())
        }
        FlowCommand::Show { artifact_id } => {
            println!("Showing flow artifact: {}", artifact_id);
            Ok(())
        }
        FlowCommand::Diff { id_a, id_b } => {
            println!("Diffing flows: {} vs {}", id_a, id_b);
            Ok(())
        }
        FlowCommand::Middleware { artifact_id } => {
            println!("Middleware trace for: {}", artifact_id);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_record_returns_ok() {
        let result = run(FlowCommand::Record {
            name: "ingestion-run-1".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn flow_diff_returns_ok() {
        let result = run(FlowCommand::Diff {
            id_a: "f001".to_string(),
            id_b: "f002".to_string(),
        });
        assert!(result.is_ok());
    }
}
