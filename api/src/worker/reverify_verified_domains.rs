use apalis::prelude::*;
use apalis_cron::Tick;

use crate::prelude::*;

pub async fn reverify_verified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	Ok(())
}
