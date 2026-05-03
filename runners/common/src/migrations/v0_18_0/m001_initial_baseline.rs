//! Baseline migration for v0.18.0.

use crate::prelude::*;

/// Baseline migration — no-op for v0.18.0 fresh installs.
#[macros::migration]
async fn migrate(_connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	Ok(())
}
