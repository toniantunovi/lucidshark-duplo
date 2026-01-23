//! Core data structures and algorithms for duplicate detection

pub mod block;
pub mod hash;
pub mod processor;
pub mod source_file;
pub mod source_line;

pub use block::Block;
// hash_line is used in tests
#[allow(unused_imports)]
pub use hash::hash_line;
pub use processor::{process_files, DuploResult};
pub use source_file::SourceFile;
pub use source_line::SourceLine;
