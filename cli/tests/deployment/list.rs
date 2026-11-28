//! `patr deployments`.

use cli::prelude::*;
use models::api::workspace::{deployment::*, runner::*};
use wiremock::{
	Mock,
	MockServer,
	matchers::{method, path},
};

use crate::{
	apply::{RUNNER_NAME, external_deployment, mount_deployment_list},
	setup,
};

/// Every deployment status, with the label the deployment tables show for it.
pub const STATUS_LABELS: [(DeploymentStatus, &str); 11] = [
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Running),
		"✅ Running",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying),
		"🟡 Deploying",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Stopped),
		"🟧 Stopped",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Errored),
		"❌ Errored",
	),
	(DeploymentStatus::Unreachable, "❓ Runner offline"),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Crashing),
		"💥 Crashing",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::HealthCheckFailing),
		"🩺 Health check failing",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::ImagePullFailed),
		"📦 Image pull failed",
	),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::InvalidArchitecture),
		"🧩 Invalid architecture",
	),
	(DeploymentStatus::Stopping, "🟠 Stopping"),
	(
		DeploymentStatus::Observed(ObservedDeploymentStatus::Unknown),
		"❔ Unknown",
	),
];

struct Ids {
	workspace: Uuid,
	runner: Uuid,
	machine_type: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000002").unwrap(),
			machine_type: Uuid::parse_str("00000000000000000000000000000003").unwrap(),
		}
	}
}

/// The ID of the deployment in [`STATUS_LABELS`] position `index`.
pub fn deployment_id(index: usize) -> Uuid {
	Uuid::parse_str(&format!("{:032x}", 0x100 + index)).unwrap()
}

/// Mount `GET /workspace/{id}/deployment` returning one deployment on
/// `runner_id` per [`STATUS_LABELS`] entry, and `GET
/// /workspace/{id}/runner/{runner_id}` for the runner name each row shows.
pub async fn mount_deployment_per_status(
	server: &MockServer,
	workspace_id: Uuid,
	runner_id: Uuid,
	machine_type: Uuid,
) {
	mount_deployment_list(
		server,
		workspace_id,
		STATUS_LABELS
			.iter()
			.enumerate()
			.map(|(index, (status, _))| {
				WithId::new(
					deployment_id(index),
					Deployment {
						status: *status,
						..external_deployment(
							&format!("deployment-{index}"),
							runner_id,
							machine_type,
							"1.27",
						)
					},
				)
			})
			.collect(),
	)
	.await;

	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{workspace_id}/runner/{runner_id}"
		)))
		.respond_with(setup::success(GetRunnerInfoResponse {
			runner: WithId::new(
				runner_id,
				Runner {
					name: RUNNER_NAME.to_string(),
					connected: true,
					last_seen: None,
					version: "0.1.0".parse().expect("valid semver"),
				},
			),
		}))
		.mount(server)
		.await;
}

/// Assert every [`STATUS_LABELS`] deployment's row in `text` shows its label.
pub fn assert_status_labels(text: &str) {
	for (index, (status, label)) in STATUS_LABELS.iter().enumerate() {
		let id = deployment_id(index).to_string();
		let row = text
			.lines()
			.find(|line| line.contains(&id))
			.unwrap_or_else(|| panic!("no row for the {status} deployment in:\n{text}"));

		assert!(
			row.contains(label),
			"the {status} deployment should show `{label}`, got: {row}"
		);
	}
}

#[tokio::test]
async fn shows_a_label_for_every_status() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_deployment_per_status(server, ids.workspace, ids.runner, ids.machine_type).await;

	let output = setup::run(setup::state(ids.workspace), &["deployments"])
		.await
		.expect("listing deployments failed");

	assert_status_labels(&output.text);
}
