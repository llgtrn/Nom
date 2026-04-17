//! Native-screen composition backend (ELF/Mach-O/PE binary generation).
//!
//! Delegates to nom-compiler's LLVM layer at runtime; this module ships
//! only the spec + validation + dispatch glue.
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetTriple { LinuxX86_64, LinuxArm64, WindowsX86_64, MacosArm64, MacosX86_64 }

impl TargetTriple {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "x86_64-unknown-linux-gnu",
            Self::LinuxArm64 => "aarch64-unknown-linux-gnu",
            Self::WindowsX86_64 => "x86_64-pc-windows-msvc",
            Self::MacosArm64 => "aarch64-apple-darwin",
            Self::MacosX86_64 => "x86_64-apple-darwin",
        }
    }
    pub fn is_linux(self) -> bool { matches!(self, Self::LinuxX86_64 | Self::LinuxArm64) }
    pub fn is_mac(self) -> bool { matches!(self, Self::MacosArm64 | Self::MacosX86_64) }
    pub fn is_windows(self) -> bool { matches!(self, Self::WindowsX86_64) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptLevel { Debug, Release, ReleaseLto }

#[derive(Clone, Debug, PartialEq)]
pub struct NativeSpec {
    pub entry_module: String,
    pub target: TargetTriple,
    pub opt_level: OptLevel,
    pub strip_symbols: bool,
    pub static_link: bool,
}

impl NativeSpec {
    pub fn new(entry_module: impl Into<String>, target: TargetTriple) -> Self {
        Self { entry_module: entry_module.into(), target, opt_level: OptLevel::Release, strip_symbols: false, static_link: false }
    }
    pub fn with_opt(mut self, opt: OptLevel) -> Self { self.opt_level = opt; self }
    pub fn stripped(mut self) -> Self { self.strip_symbols = true; self }
    pub fn statically_linked(mut self) -> Self { self.static_link = true; self }
    pub fn expected_extension(&self) -> &'static str {
        if self.target.is_windows() { "exe" } else { "" }
    }
    pub fn expected_mime(&self) -> &'static str {
        if self.target.is_windows() { "application/x-msdownload" }
        else if self.target.is_mac() { "application/x-mach-binary" }
        else { "application/x-executable" }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error("entry module must not be empty")]
    EmptyEntry,
    #[error("release-lto requires stripped symbols for reproducibility")]
    LtoWithoutStrip,
}

pub fn validate(spec: &NativeSpec) -> Result<(), NativeError> {
    if spec.entry_module.trim().is_empty() { return Err(NativeError::EmptyEntry); }
    if spec.opt_level == OptLevel::ReleaseLto && !spec.strip_symbols {
        return Err(NativeError::LtoWithoutStrip);
    }
    Ok(())
}

pub struct StubNativeScreenBackend;

impl CompositionBackend for StubNativeScreenBackend {
    fn kind(&self) -> NomKind { NomKind::ScreenNative }
    fn name(&self) -> &str { "stub-native-screen" }
    fn compose(&self, _spec: &ComposeSpec, _progress: &dyn ProgressSink, _interrupt: &InterruptFlag) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput { bytes: Vec::new(), mime_type: "application/x-executable".to_string(), cost_cents: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_triple_linux_x86_64_as_str() {
        assert_eq!(TargetTriple::LinuxX86_64.as_str(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn target_triple_all_as_str() {
        assert_eq!(TargetTriple::LinuxArm64.as_str(), "aarch64-unknown-linux-gnu");
        assert_eq!(TargetTriple::WindowsX86_64.as_str(), "x86_64-pc-windows-msvc");
        assert_eq!(TargetTriple::MacosArm64.as_str(), "aarch64-apple-darwin");
        assert_eq!(TargetTriple::MacosX86_64.as_str(), "x86_64-apple-darwin");
    }

    #[test]
    fn discriminators_is_linux() {
        assert!(TargetTriple::LinuxX86_64.is_linux());
        assert!(TargetTriple::LinuxArm64.is_linux());
        assert!(!TargetTriple::WindowsX86_64.is_linux());
        assert!(!TargetTriple::MacosArm64.is_linux());
    }

    #[test]
    fn discriminators_is_mac() {
        assert!(TargetTriple::MacosArm64.is_mac());
        assert!(TargetTriple::MacosX86_64.is_mac());
        assert!(!TargetTriple::LinuxX86_64.is_mac());
        assert!(!TargetTriple::WindowsX86_64.is_mac());
    }

    #[test]
    fn discriminators_is_windows() {
        assert!(TargetTriple::WindowsX86_64.is_windows());
        assert!(!TargetTriple::LinuxX86_64.is_windows());
        assert!(!TargetTriple::MacosArm64.is_windows());
    }

    #[test]
    fn native_spec_new_defaults() {
        let s = NativeSpec::new("main", TargetTriple::LinuxX86_64);
        assert_eq!(s.entry_module, "main");
        assert_eq!(s.target, TargetTriple::LinuxX86_64);
        assert_eq!(s.opt_level, OptLevel::Release);
        assert!(!s.strip_symbols);
        assert!(!s.static_link);
    }

    #[test]
    fn builder_chain_with_opt_stripped_statically_linked() {
        let s = NativeSpec::new("app", TargetTriple::MacosArm64)
            .with_opt(OptLevel::ReleaseLto)
            .stripped()
            .statically_linked();
        assert_eq!(s.opt_level, OptLevel::ReleaseLto);
        assert!(s.strip_symbols);
        assert!(s.static_link);
    }

    #[test]
    fn expected_extension_windows_exe() {
        let s = NativeSpec::new("app", TargetTriple::WindowsX86_64);
        assert_eq!(s.expected_extension(), "exe");
    }

    #[test]
    fn expected_extension_non_windows_empty() {
        assert_eq!(NativeSpec::new("app", TargetTriple::LinuxX86_64).expected_extension(), "");
        assert_eq!(NativeSpec::new("app", TargetTriple::MacosArm64).expected_extension(), "");
    }

    #[test]
    fn expected_mime_matches_os_family() {
        assert_eq!(NativeSpec::new("a", TargetTriple::WindowsX86_64).expected_mime(), "application/x-msdownload");
        assert_eq!(NativeSpec::new("a", TargetTriple::MacosArm64).expected_mime(), "application/x-mach-binary");
        assert_eq!(NativeSpec::new("a", TargetTriple::MacosX86_64).expected_mime(), "application/x-mach-binary");
        assert_eq!(NativeSpec::new("a", TargetTriple::LinuxX86_64).expected_mime(), "application/x-executable");
        assert_eq!(NativeSpec::new("a", TargetTriple::LinuxArm64).expected_mime(), "application/x-executable");
    }

    #[test]
    fn validate_ok_for_valid_spec() {
        let s = NativeSpec::new("main", TargetTriple::LinuxX86_64);
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_empty_entry_returns_error() {
        let s = NativeSpec::new("   ", TargetTriple::LinuxX86_64);
        assert!(matches!(validate(&s), Err(NativeError::EmptyEntry)));
    }

    #[test]
    fn validate_lto_without_strip_returns_error() {
        let s = NativeSpec::new("main", TargetTriple::LinuxX86_64)
            .with_opt(OptLevel::ReleaseLto);
        assert!(matches!(validate(&s), Err(NativeError::LtoWithoutStrip)));
    }

    #[test]
    fn validate_lto_with_strip_ok() {
        let s = NativeSpec::new("main", TargetTriple::LinuxX86_64)
            .with_opt(OptLevel::ReleaseLto)
            .stripped();
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubNativeScreenBackend;
        assert_eq!(b.kind(), NomKind::ScreenNative);
        assert_eq!(b.name(), "stub-native-screen");
    }
}
