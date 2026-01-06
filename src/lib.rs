pub mod detect;
pub mod eval;
pub mod life_core;
pub mod mutate;
pub mod search;
pub mod storage;

// Convenience re-exports for CLI consumers.
pub use detect::{DetectionConfig, Detector, DetectorStats};
pub use eval::{EvalConfig, Evaluation, Outcome};
pub use life_core::{BBox, LifeState, Seed, StepMetrics};
pub use mutate::{CompositeMutator, MutateConfig};
pub use search::{MapElitesArchive, MapElitesConfig, SearchConfig, SearchRunner, SearchSummary};
pub use storage::{rle_from_seed, ArchiveWriter, PatternRecord};
