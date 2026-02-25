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
// Keep all processor functions in public API even if not all are used in main
#[allow(unused_imports)]
pub use processor::{
    load_file_list, process_files, process_files_with_cache, process_files_with_list, DuploResult,
};
pub use source_file::SourceFile;
pub use source_line::SourceLine;
