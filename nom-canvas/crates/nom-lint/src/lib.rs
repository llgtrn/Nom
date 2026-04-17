#![deny(unsafe_code)]

pub mod diagnostic;
pub mod incremental;
pub mod registry;
pub mod rule_trait;
pub mod rules;
pub mod span;
pub mod visitor;
pub mod watcher;

pub use diagnostic::Diagnostic;
pub use rule_trait::RuleResult;
