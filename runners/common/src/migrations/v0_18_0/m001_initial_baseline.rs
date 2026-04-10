//! Baseline migration for v0.18.0.

use crate::prelude::*;

#[macros::migration]
async fn migrate(_connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	Ok(())
}
