//! `patr runner deployments`.

use cli::prelude::*;

use crate::{
	apply::{RUNNER_NAME, mount_runner},
	deployment::list::{assert_status_labels, mount_deployment_per_status},
	setup,
};

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

#[tokio::test]
async fn shows_a_label_for_every_status() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_per_status(server, ids.workspace, ids.runner, ids.machine_type).await;

	let output = setup::run(
		setup::state(ids.workspace),
		&["runner", "deployments", "--runner", RUNNER_NAME],
	)
	.await
	.expect("listing the runner's deployments failed");

	assert_status_labels(&output.text);
}
