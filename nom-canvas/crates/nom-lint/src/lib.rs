#![deny(unsafe_code)]

pub mod span;
pub mod diagnostic;
pub mod rule_trait;
pub mod registry;
pub mod visitor;
pub mod incremental;

pub use diagnostic::Diagnostic;
pub use rule_trait::RuleResult;
