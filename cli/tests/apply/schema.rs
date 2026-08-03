//! The IaaC config file schema.
//!
//! The config file is the source of truth for the resources it declares, so
//! these tests are mostly about what gets *rejected*: a file that omits
//! something the API requires, or names a field the schema doesn't have, must
//! fail rather than being quietly defaulted.
//!
//! Parsing happens before `apply` talks to the API, so the rejection cases
//! need no stub responses at all.

use cli::prelude::*;
use models::api::workspace::deployment::*;

use super::*;
use crate::setup;

/// The smallest deployment the schema accepts.
const MINIMAL: &str = r#"
- type: Deployment
  name: "test-deployment"
  image: "docker.io/library/nginx:1.27"
  runner: "test-runner"
  deploy_on_push: false
  min_horizontal_scale: 1
  max_horizontal_scale: 2
"#;

struct Ids {
	workspace: Uuid,
	runner: Uuid,
	deployment: Uuid,
	machine_type: Uuid,
	other_machine_type: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000002").unwrap(),
			deployment: Uuid::parse_str("00000000000000000000000000000003").unwrap(),
			machine_type: Uuid::parse_str("00000000000000000000000000000005").unwrap(),
			other_machine_type: Uuid::parse_str("00000000000000000000000000000006").unwrap(),
		}
	}
}

/// Apply a config file that is expected to be rejected before any request is
/// made, and hand back the error message.
async fn expect_rejected(config: &str, why: &str) -> String {
	let ids = Ids::new();
	let server = setup::reset().await;

	let error = setup::apply(setup::state(ids.workspace), config, &[])
		.await
		.expect_err(why);

	// Nothing about the file was usable, so nothing should have been asked of
	// the API either.
	assert!(
		requests(server).await.is_empty(),
		"a config file that can't be parsed shouldn't hit the API"
	);

	error.to_string()
}

/// Mount everything a valid config file needs to reach the create call, then
/// apply `config` and return the body it sent.
async fn create_body_for(config: &str) -> CreateDeploymentRequest {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_machine_types(
		server,
		ids.workspace,
		ids.other_machine_type,
		ids.machine_type,
	)
	.await;
	mount_deployment_list(server, ids.workspace, vec![]).await;
	mount_deployment_create(server, ids.workspace, ids.deployment).await;

	setup::apply(setup::state(ids.workspace), config, &[])
		.await
		.expect("apply failed");

	sole_body::<CreateDeploymentRequest>(
		server,
		"POST",
		&format!("/workspace/{}/deployment", ids.workspace),
	)
	.await
}

/// The smallest accepted file describes a complete deployment: what it leaves
/// out is genuinely empty, not "whatever happens to be there".
#[tokio::test]
async fn minimal_deployment_is_complete() {
	let body = create_body_for(MINIMAL).await;

	assert!(body.running_details.ports.is_empty());
	assert!(body.running_details.environment_variables.is_empty());
	assert!(body.running_details.config_mounts.is_empty());
	assert!(body.running_details.startup_probe.is_none());
	assert!(body.running_details.liveness_probe.is_none());
	assert!(body.running_details.volumes.is_empty());
}

#[tokio::test]
async fn ports_are_optional() {
	let body = create_body_for(&format!("{MINIMAL}  ports:\n    8080: http\n")).await;

	assert_eq!(
		body.running_details.ports,
		[(StringifiedU16::new(8080), ExposedPortType::Http)]
			.into_iter()
			.collect()
	);
}

/// Every field the API requires has to be spelled out — none of them silently
/// default to something the file never said.
#[tokio::test]
async fn required_fields_are_rejected_when_missing() {
	for field in [
		"name",
		"image",
		"runner",
		"deploy_on_push",
		"min_horizontal_scale",
		"max_horizontal_scale",
	] {
		let config = MINIMAL
			.lines()
			.filter(|line| !line.trim_start().starts_with(&format!("{field}:")))
			.collect::<Vec<_>>()
			.join("\n");

		let message = expect_rejected(
			&config,
			&format!("a config without `{field}` should be rejected"),
		)
		.await;

		assert!(
			message.contains(field),
			"the error for a missing `{field}` should name it, got: {message}"
		);
	}
}

/// Machine types aren't part of the schema. A file that still sets one is
/// rejected rather than having it silently ignored.
#[tokio::test]
async fn machine_type_is_rejected() {
	let message = expect_rejected(
		&format!("{MINIMAL}  machine_type: \"2vCPU 4GB\"\n"),
		"machine_type should be rejected",
	)
	.await;

	assert!(
		message.contains("machine_type"),
		"the error should name the unknown field, got: {message}"
	);
}

/// Unknown fields in general are rejected, so a typo can't silently do nothing.
#[tokio::test]
async fn unknown_fields_are_rejected() {
	let message = expect_rejected(
		&format!("{MINIMAL}  min_horizontal_scal: 3\n"),
		"a typo'd field should be rejected",
	)
	.await;

	assert!(
		message.contains("min_horizontal_scal"),
		"the error should name the unknown field, got: {message}"
	);
}

/// A config file is a list of resources, not a single one.
#[tokio::test]
async fn a_bare_resource_is_rejected() {
	let config = MINIMAL
		.trim_start()
		.trim_start_matches("- ")
		.replace("\n  ", "\n");

	let message = expect_rejected(&config, "a bare resource should be rejected").await;

	assert!(
		message.to_lowercase().contains("sequence"),
		"the error should say a list was expected, got: {message}"
	);
}

/// Values can be sourced from the environment rather than written inline.
///
/// `PATR_TEST_APP_NAME` is exported by `cli/tests/Justfile` — the suite can't
/// set it itself, since `std::env::set_var` is unsafe and `unsafe_code` is
/// forbidden.
#[tokio::test]
async fn values_can_come_from_the_environment() {
	let config = MINIMAL.replace(
		r#"name: "test-deployment""#,
		"name:\n    from_env: PATR_TEST_APP_NAME",
	);

	let body = create_body_for(&config).await;

	assert_eq!(body.name, "from-the-environment");
}

/// An environment variable the file refers to but that isn't set is an error,
/// not an empty string.
#[tokio::test]
async fn a_missing_environment_variable_is_an_error() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_list(server, ids.workspace, vec![]).await;

	let config = MINIMAL.replace(
		r#"name: "test-deployment""#,
		"name:\n    from_env: PATR_TEST_UNSET_VARIABLE",
	);

	let error = setup::apply(setup::state(ids.workspace), &config, &[])
		.await
		.expect_err("an unset environment variable should be an error");

	assert!(
		error.to_string().contains("PATR_TEST_UNSET_VARIABLE"),
		"the error should name the variable, got: {error}"
	);

	assert_no_writes(server).await;
}

/// The sample config shipped in `assets/` has to stay applyable — it's the
/// worked example people copy from.
#[tokio::test]
async fn the_shipped_sample_applies() {
	let sample = include_str!("../../../assets/iaac/grafana.yml");

	let body = create_body_for(sample).await;

	assert_eq!(body.name, "Grafana");
	assert_eq!(
		body.registry,
		DeploymentRegistry::ExternalRegistry {
			registry: "docker.io".to_string(),
			image_name: "grafana/grafana-oss".to_string(),
		}
	);
	assert_eq!(body.image_tag, "12");
}
