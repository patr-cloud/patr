//! Baseline migration for v0.18.0.
//!
//! This is a no-op that marks the v0.18.0 schema baseline as applied.
//! On a fresh database, the `initialize_*` functions create the current
//! schema directly; this migration exists so the tracking table records
//! that v0.18.0 has been accounted for.

use crate::prelude::*;

#[macros::migration]
async fn migrate(_connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	Ok(())
}
