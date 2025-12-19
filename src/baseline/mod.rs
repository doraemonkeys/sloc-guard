mod types;

pub use types::{
    Baseline, BaselineEntry, StructureViolationType, compute_content_hash, compute_file_hash,
    compute_hash_from_bytes, read_file_with_hash,
};

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
