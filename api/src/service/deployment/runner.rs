use std::collections::HashSet;

use sqlx::Pool;
use time::Duration;
use tokio::{sync::Mutex, task, time};

use crate::{
	db, models::db_mapping::Deployment, service, utils::Error, Database,
};

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
		let result = start_all_deployment_monitors(app.database.clone()).await;
		if let Err(error) = result {
			log::error!("Unable to start deployment monitors: {}", error.get_error());
		}
		time::sleep(Duration::from_secs(60)).await;
	}
}

async fn start_all_deployment_monitors(
	pool: Pool<Database>,
) -> Result<(), Error> {
	let mut connection = pool.acquire().await?;

	let deployments =
		db::get_deployments_in_region(&mut connection, "").await?;

	for deployment in deployments {
		task::spawn(monitor_deployment(pool.clone(), deployment));
	}

	Ok(())
}

async fn monitor_deployment(pool: Pool<Database>, deployment: Deployment) {
	let mut deployments = DEPLOYMENTS.lock().await;
	if deployments.contains(&deployment.id) {
		return;
	}
	deployments.insert(deployment.id.clone());
	drop(deployments);
	loop {
		// Monitor deployment every 10 seconds
		let result = poll_deployment(&pool, &deployment).await;
		if let Err(error) = result {
			log::error!("Unable to poll deployment: {}", error.get_error());
		}
		time::sleep(Duration::from_secs(10)).await;

		if false {
			break;
		}
	}
	let mut deployments = DEPLOYMENTS.lock().await;
	deployments.remove(&deployment.id);
	drop(deployments);
}

async fn poll_deployment(
	pool: &Pool<Database>,
	deployment: &Deployment,
) -> Result<(), Error> {
	let mut connection = pool.acquire().await?;
	let _configuration =
		service::get_deployment_config_by_id(&mut connection, &deployment.id)
			.await?;

	Ok(())
}
