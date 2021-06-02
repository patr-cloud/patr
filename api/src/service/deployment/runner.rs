use sqlx::Pool;

use crate::{service, Database};

pub async fn monitor_deployments() -> ! {
	let app = service::get_config().clone();
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

async fn start_all_deployment_monitors(pool: Pool<Database>) {
	let result = pool.acquire().await;
	if let Err(err) = result {
		log::error!(
			"Error occured while trying to aquire a database connection: {}",
			err
		);
		return;
	}
	let connection = result.unwrap();
}
