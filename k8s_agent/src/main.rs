use std::sync::Arc;

use clap::Parser;
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
	api::ListParams,
	runtime::watcher::{watcher, Event},
	Api,
	Client,
	ResourceExt,
};
use reqwest::header::AUTHORIZATION;

const DEPLOYMENT_ID_LABEL: &str = "deploymentId";
const WORKSPACE_ID_LABEL: &str = "workspaceId";
const _CI_REPO_ID_LABEL: &str = "repoId";
const _CI_BUILD_NUM_LABEL: &str = "buildNum";
const _CI_BUILD_NUM_STEP_LABEL: &str = "step";

#[derive(Debug, Clone, Parser)]
struct Args {
	// agent deployed region id
	#[arg(long, env = "PATR_REGION_ID")]
	region_id: String,

	// patr api token
	#[arg(long, env = "PATR_API_TOKEN")]
	api_token: String,

	// patr webhook url
	#[arg(long, env = "PATR_HOST", default_value = "https://api.patr.cloud")]
	patr_host: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let args = Arc::new(Args::parse());

	let client = Client::try_default().await?;

	// listening of single namespace in byoc won't work,
	// as ci will create separate namespace for each build
	let deployments = Api::<Pod>::all(client);

	// TODO: watcher may provide obsolete data,
	// need to update it to controller to get current data
	// see: https://github.com/kube-rs/kube/discussions/695#discussioncomment-1597566
	watcher(
		deployments,
		ListParams::default()
			.labels(&format!("{},{}", DEPLOYMENT_ID_LABEL, WORKSPACE_ID_LABEL)),
	)
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
	.for_each(|obj| async {
		let args = args.clone();
		// TODO: try to accumulate the events based on deployments with an
		// interval of atleast 1s to avoid increased number of status checks
		tokio::spawn(async move {
			match update_patr_deployment_status(args, obj).await {
				Ok(()) => {}
				// TODO: use log/tracing
				Err(err) => println!("{:?}", err),
			};
		});
	})
	.await;

	Ok(())
}

async fn update_patr_deployment_status(
	args: Arc<Args>,
	deployment: Pod,
) -> anyhow::Result<()> {
	let deployment_id = deployment
		.labels()
		.get(DEPLOYMENT_ID_LABEL)
		.expect("deploymentId value should be present");
	let workspace_id = deployment
		.labels()
		.get(WORKSPACE_ID_LABEL)
		.expect("workspaceId value should be present");

	println!("Triggering status check for deployment {}", deployment_id);

	reqwest::Client::new()
		.post(format!(
			"{}/workspace/{}/infrastructure/deployment/{}/check-status",
			args.patr_host, workspace_id, deployment_id
		))
		.header(AUTHORIZATION, &args.api_token)
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
