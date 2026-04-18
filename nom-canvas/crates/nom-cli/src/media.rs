pub enum MediaCommand {
    Import { path: String },
    ImportDir { path: String },
    Render { entry_hash: String, output: String },
    Transcode { input: String, codec: String },
    Diff { hash_a: String, hash_b: String },
    Similar { entry_hash: String, limit: usize },
}

pub fn run(cmd: MediaCommand) -> Result<(), String> {
    match cmd {
        MediaCommand::Import { path } => {
            println!("Importing media: {}", path);
            Ok(())
        }
        MediaCommand::ImportDir { path } => {
            println!("Importing media directory: {}", path);
            Ok(())
        }
        MediaCommand::Render { entry_hash, output } => {
            println!("Rendering {} → {}", entry_hash, output);
            Ok(())
        }
        MediaCommand::Transcode { input, codec } => {
            println!("Transcoding {} with codec {}", input, codec);
            Ok(())
        }
        MediaCommand::Diff { hash_a, hash_b } => {
            println!("Diffing media: {} vs {}", hash_a, hash_b);
            Ok(())
        }
        MediaCommand::Similar { entry_hash, limit } => {
            println!("Finding {} similar to {}", limit, entry_hash);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_import_returns_ok() {
        let result = run(MediaCommand::Import {
            path: "/tmp/clip.mp4".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn media_similar_returns_ok() {
        let result = run(MediaCommand::Similar {
            entry_hash: "deadbeef".to_string(),
            limit: 10,
        });
        assert!(result.is_ok());
    }
}
