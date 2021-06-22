use std::collections::HashSet;

use futures::FutureExt;
use sqlx::Pool;
use time::Duration;
use tokio::{sync::Mutex, task, time};

use crate::{db, service, utils::Error, Database};

lazy_static::lazy_static! {
	static ref DEPLOYMENTS: Mutex<HashSet<Vec<u8>>> = Mutex::new(HashSet::new());
}

pub async fn monitor_deployments() {
	let app = service::get_app().clone();
	let mut should_exit = task::spawn(tokio::signal::ctrl_c());
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
			log::error!(
				"Unable to start deployment monitors: {}",
				error.get_error()
			);
		}
		for _ in 0..60 {
			time::sleep(Duration::from_secs(1)).await;
			if (&mut should_exit).now_or_never().is_some() {
				return;
			}
		}
	}
}

async fn start_all_deployment_monitors(
	pool: Pool<Database>,
) -> Result<(), Error> {
	let mut connection = pool.acquire().await?;

	let deployments = db::get_all_deployments(&mut connection).await?;
	// TODO change this to "get all deployments that are not already running",
	// which will get deployments that haven't been updated in the last 10
	// seconds.
	let running_deployments = DEPLOYMENTS.lock().await;
	for deployment in deployments {
		if !running_deployments.contains(&deployment.id) {
			task::spawn(monitor_deployment(pool.clone(), deployment.id));
		}
	}
	drop(running_deployments);

	Ok(())
}

async fn monitor_deployment(pool: Pool<Database>, deployment_id: Vec<u8>) {
	loop {
		// Monitor deployment every 10 seconds
		time::sleep(Duration::from_secs(10)).await;

		let mut connection = match pool.acquire().await {
			Ok(connection) => connection,
			Err(error) => {
				log::error!("Unable to aquire db connection: {}", error);
				continue;
			}
		};
		let result =
			db::get_deployment_by_id(&mut connection, &deployment_id).await;
		let deployment = match result {
			Ok(deployment) => deployment,
			Err(error) => {
				log::error!("Unable to get deployment: {}", error);
				continue;
			}
		};
		if let Some(deployment) = deployment {
			// TODO check if the `deployment` is doing okay
		} else {
			// If the deployment doesn't exist in the DB, stop monitoring
			log::info!("Deployment `{}` doesn't exist in the database anymore. Will stop monitoring it.", hex::encode(&deployment_id));
			break;
		}
		drop(connection);
	}
	let mut deployments = DEPLOYMENTS.lock().await;
	deployments.remove(&deployment_id);
	drop(deployments);
}
