//! Event processing and metrics computation

pub mod achievements;
pub mod backfill;
pub mod categorize;
pub mod metrics;
pub mod recalculate;
pub mod scores;
pub mod sessions;
pub mod sync;

pub use backfill::{BackfillError, Backfiller};
pub use categorize::{
    categorize_batch, get_stats as get_category_stats, CategorizeError, CategorizeStats,
    CategoryStats,
};
pub use recalculate::{recalculate_all_xp, RecalculationStats};
pub use sync::{SyncConfig, SyncService};
