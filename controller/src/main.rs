#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This project generates a controller that can be installed in the user's
//! Kubernetes cluster. Each controller will be responsible for managing the
//! respective cluster. The controller will periodically check with the API and
//! make sure that the cluster's state is up to date with the API's state.

use std::{sync::Arc, time::Duration};

use app::AppState;
use futures::{future, FutureExt, StreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
	api::ListParams,
	runtime::{controller::Action, watcher, Controller, WatchStreamExt},
	Api,
	Client,
	Config,
	ResourceExt,
};

mod app;

#[tokio::main]
async fn main() {
	let state = AppState::try_default();

	let client = Client::try_default()
		.await
		.expect("Failed to get kubernetes client details");

	sync_deployments(client.clone()).await.unwrap();

	let pods = Api::<Deployment>::all(client.clone());

	Controller::new(pods, watcher::Config::default())
		.run(reconcile, error_policy, Arc::new(state))
		.for_each(|_| future::ready(()))
		.await;
}

async fn reconcile(obj: Arc<Deployment>, _ctx: Arc<AppState>) -> Result<Action, kube::Error> {
	println!("{}", serde_json::to_string_pretty(&obj).unwrap());
	Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Deployment>, _err: &kube::Error, _ctx: Arc<AppState>) -> Action {
	Action::requeue(Duration::from_secs(5))
}

async fn sync_deployments(client: Client) -> Result<(), kube::Error> {
	let mut current_running_deployments = Api::<Deployment>::all(client.clone())
		.list(&ListParams::default().timeout(u32::MAX).labels(
			if cfg!(debug_assertions) {
				""
			} else {
				"cloud.patr.managed-by-patr=true"
			},
		))
		.await?
		.items;
	let mut should_be_running_deployments = Api::<Deployment>::all(client)
		.list(&ListParams::default().timeout(u32::MAX).labels(
			if cfg!(debug_assertions) {
				""
			} else {
				"cloud.patr.managed-by-patr=true"
			},
		))
		.await?
		.items;

	for (current_running, should_be_running) in current_running_deployments
		.into_iter()
		.zip(should_be_running_deployments.into_iter())
	{
		println!("deployment: {}", current_running.name_any());
	}

	Ok(())
}
