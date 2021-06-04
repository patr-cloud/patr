use std::collections::HashSet;

use sqlx::Pool;
use tokio::sync::Mutex;

use crate::{db, service, utils::Error, Database};

lazy_static::lazy_static! {
	static ref DEPLOYMENTS: Mutex<HashSet<Vec<u8>>> = Mutex::new(HashSet::new());
}

pub async fn monitor_deployments() -> ! {
	let app = service::get_app().clone();
	loop {
		// Db stores the PRIMARY KEY(IP address, region) as server details
		// Each server has it's own "alive" status independent of the runner's
		// "alive" status. If a server's "alive" status isn't updated in a long
		// time, any runners in the same region can take over

		// Find the list of servers running on the cloud provider
		// Find the list of servers registered with the db
		// Register any new server and unregister any old server

		// Find the list of deployments to deploy.
		// Then start deployment monitor for each of them
		// Then wait for, idk like 1 minute
	}
}

async fn start_all_deployment_monitors(
	pool: Pool<Database>,
) -> Result<(), Error> {
	let mut connection = pool.acquire().await?;

	let deployments =
		db::get_deployments_in_region(&mut connection, "").await?;

	for deployment in deployments {}

	Ok(())
}
