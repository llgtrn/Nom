//! nom-runtime: Runtime library for compiled Nom binaries.
//!
//! Provides the runtime functions that nom-llvm's generated LLVM IR calls:
//! - String operations (create, concat, compare, length)
//! - Print/println
//! - Memory allocation
//! - File I/O

mod string;
mod print;
mod alloc;
mod io;
mod list;

// Re-export all extern "C" functions
pub use string::*;
pub use print::*;
pub use alloc::*;
pub use io::*;
pub use list::*;
