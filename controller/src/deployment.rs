use std::{sync::Arc, time::Duration};

use futures::{future, FutureExt, StreamExt};
use k8s_openapi::api::{
	apps::v1::{Deployment, StatefulSet},
	autoscaling::v2::HorizontalPodAutoscaler,
	core::v1::{ConfigMap, PersistentVolumeClaim, Service},
	networking::v1::Ingress,
};
use kube::{
	runtime::{controller::Action, watcher, Controller},
	Api,
	Client,
};
use tokio::signal;

use crate::{app::AppState, models::PatrDeployment};

pub async fn start_controller(client: Client, state: Arc<AppState>) {
	Controller::new(
		Api::<PatrDeployment>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Deployment>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<StatefulSet>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<ConfigMap>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Service>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<HorizontalPodAutoscaler>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<PersistentVolumeClaim>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Ingress>::all(client.clone()),
		watcher::Config::default(),
	)
	.graceful_shutdown_on(signal::ctrl_c().map(|_| ()))
	.run(reconcile, error_policy, state.clone())
	.for_each(|_| future::ready(()))
	.await;
}

async fn reconcile(_obj: Arc<PatrDeployment>, _ctx: Arc<AppState>) -> Result<Action, kube::Error> {
	Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_obj: Arc<PatrDeployment>, _err: &kube::Error, _ctx: Arc<AppState>) -> Action {
	Action::requeue(Duration::from_secs(5))
}
