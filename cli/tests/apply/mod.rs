//! Tests for `patr apply`.

pub mod deployment;
pub mod domain;
pub mod managed_url;
pub mod schema;

use std::collections::BTreeMap;

use cli::prelude::*;
use models::api::workspace::{deployment::*, runner::*};
use wiremock::{
	Mock,
	MockServer,
	Request,
	matchers::{method, path},
};

use crate::setup;

/// A runner the config files in these tests refer to by name.
pub const RUNNER_NAME: &str = "test-runner";

/// Mount `GET /workspace/{id}/runner` returning a single runner.
pub async fn mount_runner(server: &MockServer, workspace_id: Uuid, runner_id: Uuid) {
	Mock::given(method("GET"))
		.and(path(format!("/workspace/{workspace_id}/runner")))
		.respond_with(setup::success_list(
			ListRunnersForWorkspaceResponse {
				runners: vec![WithId::new(
					runner_id,
					Runner {
						name: RUNNER_NAME.to_string(),
						connected: true,
						last_seen: None,
					},
				)],
			},
			1,
		))
		.mount(server)
		.await;
}

/// Mount `GET /workspace/{id}/deployment/machine-type`.
///
/// The first entry is deliberately *not* the one existing deployments in these
/// tests run on, so a carried-over machine type is distinguishable from the
/// list-first fallback the create path uses.
pub async fn mount_machine_types(server: &MockServer, workspace_id: Uuid, first: Uuid, rest: Uuid) {
	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{workspace_id}/deployment/machine-type"
		)))
		.respond_with(setup::success(ListAllDeploymentMachineTypeResponse {
			machine_types: vec![
				WithId::new(
					first,
					DeploymentMachineType {
						cpu_count: 1,
						memory_count: 4,
					},
				),
				WithId::new(
					rest,
					DeploymentMachineType {
						cpu_count: 4,
						memory_count: 32,
					},
				),
			],
		}))
		.mount(server)
		.await;
}

/// Mount `GET /workspace/{id}/deployment` returning `deployments`.
pub async fn mount_deployment_list(
	server: &MockServer,
	workspace_id: Uuid,
	deployments: Vec<WithId<Deployment>>,
) {
	let total = deployments.len();

	Mock::given(method("GET"))
		.and(path(format!("/workspace/{workspace_id}/deployment")))
		.respond_with(setup::success_list(
			ListDeploymentResponse { deployments },
			total,
		))
		.mount(server)
		.await;
}

/// Mount `GET /workspace/{id}/deployment/{deployment_id}`.
pub async fn mount_deployment_info(
	server: &MockServer,
	workspace_id: Uuid,
	deployment: WithId<Deployment>,
	running_details: DeploymentRunningDetails,
) {
	let deployment_id = deployment.id;

	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{workspace_id}/deployment/{deployment_id}"
		)))
		.respond_with(setup::success(GetDeploymentInfoResponse {
			deployment,
			running_details,
		}))
		.mount(server)
		.await;
}

/// Mount `PATCH /workspace/{id}/deployment/{deployment_id}`.
pub async fn mount_deployment_update(server: &MockServer, workspace_id: Uuid, deployment_id: Uuid) {
	Mock::given(method("PATCH"))
		.and(path(format!(
			"/workspace/{workspace_id}/deployment/{deployment_id}"
		)))
		.respond_with(setup::success(UpdateDeploymentResponse))
		.mount(server)
		.await;
}

/// Mount `POST /workspace/{id}/deployment`.
pub async fn mount_deployment_create(server: &MockServer, workspace_id: Uuid, deployment_id: Uuid) {
	Mock::given(method("POST"))
		.and(path(format!("/workspace/{workspace_id}/deployment")))
		.respond_with(setup::success(CreateDeploymentResponse {
			id: OnlyId::only_id(deployment_id),
		}))
		.mount(server)
		.await;
}

/// An external-registry deployment, so tests don't need the Patr
/// container-repository lookup.
pub fn external_deployment(
	name: &str,
	runner_id: Uuid,
	machine_type: Uuid,
	image_tag: &str,
) -> Deployment {
	Deployment {
		name: name.to_string(),
		registry: DeploymentRegistry::ExternalRegistry {
			registry: "docker.io".to_string(),
			image_name: "library/nginx".to_string(),
		},
		image_tag: image_tag.to_string(),
		status: DeploymentStatus::Running,
		runner: runner_id,
		machine_type,
		current_live_digest: None,
	}
}

/// Running details with a volume attached — the thing the IaaC schema can't
/// describe and apply therefore has to preserve.
pub fn running_details_with_volume(volume_id: Uuid) -> DeploymentRunningDetails {
	DeploymentRunningDetails {
		deploy_on_push: false,
		min_horizontal_scale: 1,
		max_horizontal_scale: 3,
		ports: [(StringifiedU16::new(8080), ExposedPortType::Http)]
			.into_iter()
			.collect(),
		environment_variables: BTreeMap::new(),
		startup_probe: None,
		liveness_probe: None,
		config_mounts: BTreeMap::new(),
		volumes: [(volume_id, "/data".to_string())].into_iter().collect(),
	}
}

/// Every request the stub server saw, in order.
pub async fn requests(server: &MockServer) -> Vec<Request> {
	server
		.received_requests()
		.await
		.expect("the stub server isn't recording requests")
}

/// The body of the single request matching `wanted_method` + `wanted_path`,
/// deserialized. Panics if there isn't exactly one.
pub async fn sole_body<T: serde::de::DeserializeOwned>(
	server: &MockServer,
	wanted_method: &str,
	wanted_path: &str,
) -> T {
	let matching = requests(server)
		.await
		.into_iter()
		.filter(|req| req.method.as_str() == wanted_method && req.url.path() == wanted_path)
		.collect::<Vec<_>>();

	assert_eq!(
		matching.len(),
		1,
		"expected exactly one {wanted_method} {wanted_path}, got {}",
		matching.len()
	);

	matching[0]
		.body_json::<T>()
		.expect("failed to deserialize the request body")
}

/// Assert the stub server saw no mutating request.
pub async fn assert_no_writes(server: &MockServer) {
	let writes = requests(server)
		.await
		.into_iter()
		.filter(|req| matches!(req.method.as_str(), "POST" | "PATCH" | "PUT" | "DELETE"))
		.map(|req| format!("{} {}", req.method, req.url.path()))
		.collect::<Vec<_>>();

	assert!(
		writes.is_empty(),
		"expected no mutating requests, got: {writes:?}"
	);
}
