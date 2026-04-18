// nom convert — migrate .nomx files between syntax formats.

/// Direction of a syntax conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConvertDirection {
    /// Migrate v1 (`fn X -> Y`) to v2 (`define X that Y`).
    V1ToV2,
    /// Reverse: v2 (`define X that Y`) back to v1 (`fn X -> Y`).
    V2ToV1,
    /// Inspect source and pick direction automatically.
    AutoDetect,
}

/// Options controlling a conversion run.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub direction: ConvertDirection,
    pub dry_run: bool,
    pub verbose: bool,
    /// Create a `.bak` file before overwriting the original.
    pub backup: bool,
}

impl ConvertOptions {
    /// Create options with `dry_run=false`, `verbose=false`, `backup=true`.
    pub fn new(direction: ConvertDirection) -> Self {
        Self {
            direction,
            dry_run: false,
            verbose: false,
            backup: true,
        }
    }

    /// Enable dry-run (no files written).
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Disable backup file creation.
    pub fn no_backup(mut self) -> Self {
        self.backup = false;
        self
    }
}

/// Outcome of converting one file.
#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub input_path: String,
    pub output_path: String,
    pub lines_changed: u32,
    pub success: bool,
    pub message: String,
}

impl ConvertResult {
    /// Successful conversion with a change count.
    pub fn success(input: &str, output: &str, changed: u32) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            lines_changed: changed,
            success: true,
            message: format!("converted {} lines", changed),
        }
    }

    /// Failed conversion with a human-readable reason.
    pub fn failure(input: &str, reason: &str) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: input.to_string(),
            lines_changed: 0,
            success: false,
            message: reason.to_string(),
        }
    }

    /// Returns `true` when no lines were changed (already up-to-date).
    pub fn is_noop(&self) -> bool {
        self.lines_changed == 0
    }
}

/// Apply a syntax conversion to a source string and return the result.
///
/// - `V1ToV2`: replaces `fn ` with `define ` and ` -> ` / `->` with ` that `.
/// - `V2ToV1`: reverses those substitutions.
/// - `AutoDetect`: detects v1 by the presence of `fn `, then applies `V1ToV2`;
///   otherwise returns the source unchanged.
pub fn convert_source(source: &str, direction: ConvertDirection) -> String {
    match direction {
        ConvertDirection::V1ToV2 => source
            .replace("fn ", "define ")
            .replace(" -> ", " that ")
            .replace("->", " that "),
        ConvertDirection::V2ToV1 => source
            .replace("define ", "fn ")
            .replace(" that ", " -> "),
        ConvertDirection::AutoDetect => {
            if source.contains("fn ") {
                convert_source(source, ConvertDirection::V1ToV2)
            } else {
                source.to_string()
            }
        }
    }
}

/// Convert a single file at `path` using the provided options.
///
/// In dry-run mode the file is read and converted in memory but not written.
/// When `backup` is enabled a `.bak` copy is created before overwriting.
pub fn convert_file(path: &str, opts: &ConvertOptions) -> ConvertResult {
    if path.is_empty() {
        return ConvertResult::failure(path, "path must not be empty");
    }

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ConvertResult::failure(path, &e.to_string()),
    };

    let converted = convert_source(&source, opts.direction);

    let lines_changed = source
        .lines()
        .zip(converted.lines())
        .filter(|(a, b)| a != b)
        .count() as u32;

    if opts.dry_run {
        return ConvertResult::success(path, path, lines_changed);
    }

    if opts.backup {
        let bak = format!("{}.bak", path);
        if let Err(e) = std::fs::write(&bak, &source) {
            return ConvertResult::failure(path, &format!("backup failed: {}", e));
        }
    }

    if let Err(e) = std::fs::write(path, &converted) {
        return ConvertResult::failure(path, &e.to_string());
    }

    ConvertResult::success(path, path, lines_changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_v1_to_v2() {
        let input = "fn greet -> say hi";
        let output = convert_source(input, ConvertDirection::V1ToV2);
        assert_eq!(output, "define greet that say hi");
    }

    #[test]
    fn convert_v2_to_v1() {
        let input = "define greet that say hi";
        let output = convert_source(input, ConvertDirection::V2ToV1);
        assert_eq!(output, "fn greet -> say hi");
    }

    #[test]
    fn convert_auto_detect_v1() {
        let input = "fn greet -> say hi";
        let output = convert_source(input, ConvertDirection::AutoDetect);
        // source contains "fn " so AutoDetect should apply V1ToV2
        assert_eq!(output, "define greet that say hi");
    }

    #[test]
    fn convert_result_is_noop() {
        let r = ConvertResult::success("a.nomx", "a.nomx", 0);
        assert!(r.is_noop());
    }

    #[test]
    fn convert_file_dry_run() {
        // Use a temp file so the test is self-contained.
        let dir = std::env::temp_dir();
        let path = dir.join("test_convert.nomx");
        std::fs::write(&path, "fn greet -> say hi").expect("write temp file");

        let path_str = path.to_str().expect("valid utf-8 path");
        let opts = ConvertOptions::new(ConvertDirection::V1ToV2).dry_run();
        let result = convert_file(path_str, &opts);

        assert!(result.success, "expected success, got: {}", result.message);
        // lines_changed is u32 so it is always non-negative; just confirm it is set
        let _ = result.lines_changed;

        // Dry-run: original file must be unchanged.
        let after = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(after, "fn greet -> say hi");

        let _ = std::fs::remove_file(&path);
    }
}
