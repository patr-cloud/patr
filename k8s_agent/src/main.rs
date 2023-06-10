use futures::{future, StreamExt};
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Pod};
use kube::{
	api::ListParams,
	runtime::watcher::{watcher, Event},
	Api,
	Client,
	ResourceExt,
};
use once_cell_regex::{exports::regex::Regex, regex};

const DEPLOYMENT_ID_LABEL: &str = "deploymentId";
const WORKSPACE_ID_LABEL: &str = "workspaceId";
const _REGION_ID_LABEL: &str = "regionId";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let uuid_pattern: &Regex = regex!("[a-f0-9]{32}");

	let client = Client::try_default().await?;

	// listening of single namespace in byoc won't work,
	// as ci will created separate namespace for each build
	let deployments = Api::<Deployment>::all(client);
	// TODO: need to add support for sts as it is used by patr

	// TODO: watcher may provide obsolete data,
	// need to update it to controller to get current data
	// see: https://github.com/kube-rs/kube/discussions/695#discussioncomment-1597566
	watcher(deployments, ListParams::default())
		.filter_map(|event| future::ready(event.ok()))
		.filter_map(|event| async {
			match event {
				Event::Applied(obj) => Some(obj),
				// TODO: only added/modified events are watched
				// if the watcher restarted, the missed events will come
				// in restarted type, need to handle that also
				_ => None,
			}
		})
		.filter(|obj| {
			future::ready(
				obj.metadata
					.namespace
					.as_deref()
					.map_or(false, |ns| uuid_pattern.is_match(ns)),
			)
		})
		.for_each_concurrent(16, |obj| async move {
			match update_patr_deployment_status(obj).await {
				Ok(()) => {}
				// TODO: use log/tracing
				Err(err) => println!("{:?}", err),
			};
		})
		.await;

	Ok(())
}

async fn update_patr_deployment_status(
	deployment: Deployment,
) -> anyhow::Result<()> {
	let Some(deployment_id) = deployment.labels().get(DEPLOYMENT_ID_LABEL) else {
		return Ok(())
	};
	let Some(workspace_id) = deployment.labels().get(WORKSPACE_ID_LABEL) else {
		return Ok(())
	};

	let pods =
		Api::<Pod>::namespaced(Client::try_default().await?, workspace_id)
			.list(
				&ListParams::default()
					.labels(&format!("deploymentId={}", deployment_id)),
			)
			.await?;
	let mut container_status = pods
		.items
		.into_iter()
		.filter_map(|pod| pod.status)
		.filter_map(|status| status.container_statuses)
		.flat_map(|status| {
			status
				.into_iter()
				.filter_map(|status| status.state)
				.filter_map(|state| state.waiting)
				.filter_map(|waiting| waiting.reason)
				.collect::<Vec<_>>()
		});

	if container_status.any(|status| {
		status.contains(&"ErrImagePull".to_string()) ||
			status.contains(&"CrashLoopBackOff".to_string())
	}) {
		// notify patr that deployment errored
		// and return
	}

	let Some(replicas) = deployment.status.and_then(|status| status.ready_replicas) else {
		return Ok(())
	};

	// TODO: use hpa api to get min replica count
	let min_replica_needed = 1;

	if replicas < min_replica_needed {
		// notify patr that deployment is still happening
		// and return
	}

	// notify patr that deployment is running
	// and return

	Ok(())
}
