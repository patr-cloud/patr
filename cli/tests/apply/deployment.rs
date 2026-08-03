//! `patr apply` against deployment resources.
//!
//! The update route takes the whole object and rewrites every field, so these
//! tests mostly assert on the exact body the CLI sends — that is where the
//! "apply reverts things the config file doesn't mention" class of bug lives.

use cli::prelude::*;
use models::api::workspace::deployment::*;

use super::*;
use crate::setup;

/// A config file describing the deployment the tests operate on. Note it says
/// nothing about machine types or volumes — the IaaC schema has no such
/// fields.
const CONFIG: &str = r#"
- type: Deployment
  name: "test-deployment"
  image: "docker.io/library/nginx:1.27"
  runner: "test-runner"
  deploy_on_push: false
  min_horizontal_scale: 2
  max_horizontal_scale: 5
  ports:
    8080: http
  env:
    LOG_LEVEL: debug
"#;

/// Fixed IDs so assertions can name them.
struct Ids {
	workspace: Uuid,
	runner: Uuid,
	deployment: Uuid,
	volume: Uuid,
	/// The machine type the existing deployment runs on.
	machine_type: Uuid,
	/// The machine type `ListAllDeploymentMachineType` returns first.
	other_machine_type: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000002").unwrap(),
			deployment: Uuid::parse_str("00000000000000000000000000000003").unwrap(),
			volume: Uuid::parse_str("00000000000000000000000000000004").unwrap(),
			machine_type: Uuid::parse_str("00000000000000000000000000000005").unwrap(),
			other_machine_type: Uuid::parse_str("00000000000000000000000000000006").unwrap(),
		}
	}
}

/// Mount everything the update path reads, for a deployment that already
/// exists with a volume attached and a non-list-first machine type.
async fn mount_existing(ids: &Ids) -> &'static wiremock::MockServer {
	let server = setup::reset().await;

	let existing = WithId::new(
		ids.deployment,
		external_deployment("test-deployment", ids.runner, ids.machine_type, "1.26"),
	);

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_machine_types(
		server,
		ids.workspace,
		ids.other_machine_type,
		ids.machine_type,
	)
	.await;
	mount_deployment_list(server, ids.workspace, vec![existing.clone()]).await;
	mount_deployment_info(
		server,
		ids.workspace,
		existing,
		running_details_with_volume(ids.volume),
	)
	.await;
	mount_deployment_update(server, ids.workspace, ids.deployment).await;

	server
}

/// The regression this whole change exists for: applying a config file that
/// says nothing about volumes must not detach them, and must not move the
/// deployment onto a different machine type.
#[tokio::test]
async fn update_preserves_volumes_and_machine_type() {
	let ids = Ids::new();
	let server = mount_existing(&ids).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<UpdateDeploymentRequest>(
		server,
		"PATCH",
		&format!("/workspace/{}/deployment/{}", ids.workspace, ids.deployment),
	)
	.await;

	assert_eq!(
		body.running_details.volumes,
		[(ids.volume, "/data".to_string())]
			.into_iter()
			.collect::<std::collections::BTreeMap<_, _>>(),
		"apply detached the volume the config file doesn't describe"
	);
	assert_eq!(
		body.machine_type, ids.machine_type,
		"apply moved the deployment off its machine type"
	);
}

/// Everything the config file *does* declare is applied verbatim.
#[tokio::test]
async fn update_applies_declared_fields() {
	let ids = Ids::new();
	let server = mount_existing(&ids).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<UpdateDeploymentRequest>(
		server,
		"PATCH",
		&format!("/workspace/{}/deployment/{}", ids.workspace, ids.deployment),
	)
	.await;

	assert_eq!(body.name, "test-deployment");
	assert_eq!(body.image_tag, "1.27", "the new image tag wasn't applied");
	assert_eq!(body.runner, ids.runner);
	assert!(!body.running_details.deploy_on_push);
	assert_eq!(body.running_details.min_horizontal_scale, 2);
	assert_eq!(body.running_details.max_horizontal_scale, 5);
	assert_eq!(
		body.running_details.ports,
		[(StringifiedU16::new(8080), ExposedPortType::Http)]
			.into_iter()
			.collect()
	);
	assert_eq!(
		body.running_details
			.environment_variables
			.get("LOG_LEVEL")
			.and_then(EnvironmentVariableValue::value)
			.map(String::as_str),
		Some("debug")
	);
}

/// Applying the same file twice must send the same thing both times.
#[tokio::test]
async fn update_is_idempotent() {
	let ids = Ids::new();
	let server = mount_existing(&ids).await;
	let state = setup::state(ids.workspace);

	setup::apply(state.clone(), CONFIG, &[])
		.await
		.expect("first apply failed");
	setup::apply(state, CONFIG, &[])
		.await
		.expect("second apply failed");

	let update_path = format!("/workspace/{}/deployment/{}", ids.workspace, ids.deployment);
	let bodies = requests(server)
		.await
		.into_iter()
		.filter(|req| req.method.as_str() == "PATCH" && req.url.path() == update_path)
		.map(|req| req.body_json::<serde_json::Value>().unwrap())
		.collect::<Vec<_>>();

	assert_eq!(bodies.len(), 2, "expected two updates");
	assert_eq!(bodies[0], bodies[1], "apply isn't idempotent");
}

/// The update route has no registry field, so a config file that moves the
/// image to another registry has to fail loudly rather than silently applying
/// only the tag.
#[tokio::test]
async fn update_rejects_a_registry_change() {
	let ids = Ids::new();
	let server = setup::reset().await;

	// The deployment is on ghcr.io; the config file says docker.io.
	let existing = WithId::new(
		ids.deployment,
		Deployment {
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "ghcr.io".to_string(),
				image_name: "library/nginx".to_string(),
			},
			..external_deployment("test-deployment", ids.runner, ids.machine_type, "1.26")
		},
	);

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_list(server, ids.workspace, vec![existing.clone()]).await;
	mount_deployment_info(
		server,
		ids.workspace,
		existing,
		running_details_with_volume(ids.volume),
	)
	.await;
	mount_deployment_update(server, ids.workspace, ids.deployment).await;

	let error = setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect_err("apply should have rejected the registry change");

	let message = error.to_string();
	assert!(
		message.contains("ghcr.io") && message.contains("docker.io"),
		"the error should name both registries, got: {message}"
	);

	assert_no_writes(server).await;
}

/// A Patr-registry deployment is named in the error the way a user would write
/// it — `registry/workspace/repository`, not the repository's UUID.
#[tokio::test]
async fn registry_change_error_spells_out_a_patr_repository() {
	let ids = Ids::new();
	let server = setup::reset().await;

	let repository_id = Uuid::parse_str("00000000000000000000000000000007").unwrap();

	// The deployment is on Patr's registry; the config file says docker.io.
	let existing = WithId::new(
		ids.deployment,
		Deployment {
			registry: DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id,
			},
			..external_deployment("test-deployment", ids.runner, ids.machine_type, "1.26")
		},
	);

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_list(server, ids.workspace, vec![existing.clone()]).await;
	mount_deployment_info(
		server,
		ids.workspace,
		existing,
		running_details_with_volume(ids.volume),
	)
	.await;
	mount_repository_info(server, ids.workspace, repository_id, "my-app").await;

	let error = setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect_err("apply should have rejected the registry change");

	let message = error.to_string();
	assert!(
		message.contains(&format!("registry.patr.cloud/{}/my-app", ids.workspace)),
		"the Patr registry should be spelled out as registry/workspace/repository, got: {message}"
	);
	assert!(
		!message.contains(&repository_id.to_string()),
		"the repository UUID shouldn't leak into the message, got: {message}"
	);

	assert_no_writes(server).await;
}

/// A Patr image is written `registry.patr.cloud/{workspace}/{repo}`, but a
/// repository's name is only the part after the workspace — the workspace
/// segment has to be stripped before looking it up.
#[tokio::test]
async fn patr_image_resolves_the_repository_after_the_workspace() {
	let ids = Ids::new();
	let server = setup::reset().await;

	let repository_id = Uuid::parse_str("00000000000000000000000000000007").unwrap();

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_machine_types(
		server,
		ids.workspace,
		ids.other_machine_type,
		ids.machine_type,
	)
	.await;
	mount_deployment_list(server, ids.workspace, vec![]).await;
	mount_repository_list(server, ids.workspace, repository_id, "my-app").await;
	mount_deployment_create(server, ids.workspace, ids.deployment).await;

	let config = CONFIG.replace(
		r#"image: "docker.io/library/nginx:1.27""#,
		&format!(
			r#"image: "registry.patr.cloud/{}/my-app:1.27""#,
			ids.workspace
		),
	);

	setup::apply(setup::state(ids.workspace), &config, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<CreateDeploymentRequest>(
		server,
		"POST",
		&format!("/workspace/{}/deployment", ids.workspace),
	)
	.await;

	assert_eq!(
		body.registry,
		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id,
		}
	);
	assert_eq!(body.image_tag, "1.27");
}

/// An image belonging to another workspace can't be resolved here, and saying
/// so beats letting the repository lookup fail with a confusing name.
#[tokio::test]
async fn patr_image_from_another_workspace_is_rejected() {
	let ids = Ids::new();
	let server = setup::reset().await;

	let other_workspace = Uuid::parse_str("000000000000000000000000000000ff").unwrap();
	let repository_id = Uuid::parse_str("00000000000000000000000000000007").unwrap();

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_list(server, ids.workspace, vec![]).await;
	mount_repository_list(server, ids.workspace, repository_id, "my-app").await;

	let config = CONFIG.replace(
		r#"image: "docker.io/library/nginx:1.27""#,
		&format!(r#"image: "registry.patr.cloud/{other_workspace}/my-app:1.27""#),
	);

	let error = setup::apply(setup::state(ids.workspace), &config, &[])
		.await
		.expect_err("apply should have rejected another workspace's image");

	let message = error.to_string();
	assert!(
		message.contains(&other_workspace.to_string()) &&
			message.contains(&ids.workspace.to_string()),
		"the error should name both workspaces, got: {message}"
	);

	assert_no_writes(server).await;
}

/// Repository names can contain slashes, so a leading segment that isn't a
/// workspace ID stays part of the name.
#[tokio::test]
async fn patr_image_keeps_a_slashed_repository_name() {
	let ids = Ids::new();
	let server = setup::reset().await;

	let repository_id = Uuid::parse_str("00000000000000000000000000000007").unwrap();

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_machine_types(
		server,
		ids.workspace,
		ids.other_machine_type,
		ids.machine_type,
	)
	.await;
	mount_deployment_list(server, ids.workspace, vec![]).await;
	mount_repository_list(server, ids.workspace, repository_id, "team/my-app").await;
	mount_deployment_create(server, ids.workspace, ids.deployment).await;

	let config = CONFIG.replace(
		r#"image: "docker.io/library/nginx:1.27""#,
		r#"image: "registry.patr.cloud/team/my-app:1.27""#,
	);

	setup::apply(setup::state(ids.workspace), &config, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<CreateDeploymentRequest>(
		server,
		"POST",
		&format!("/workspace/{}/deployment", ids.workspace),
	)
	.await;

	assert_eq!(
		body.registry,
		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id,
		}
	);
}

/// With no matching deployment, apply creates one and asks for it to be
/// deployed straight away.
#[tokio::test]
async fn create_when_no_deployment_matches() {
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

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<CreateDeploymentRequest>(
		server,
		"POST",
		&format!("/workspace/{}/deployment", ids.workspace),
	)
	.await;

	assert_eq!(body.name, "test-deployment");
	assert!(body.deploy_on_create);
	assert_eq!(
		body.registry,
		DeploymentRegistry::ExternalRegistry {
			registry: "docker.io".to_string(),
			image_name: "library/nginx".to_string(),
		}
	);
	// Nothing declares a machine type, so a new deployment lands on the first
	// one the workspace lists.
	assert_eq!(body.machine_type, ids.other_machine_type);
	assert!(body.running_details.volumes.is_empty());
}

/// A dry run validates the file and resolves every reference, but must not
/// write anything.
#[tokio::test]
async fn dry_run_does_not_write() {
	let ids = Ids::new();
	let server = mount_existing(&ids).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &["--dry-run"])
		.await
		.expect("dry run failed");

	assert_no_writes(server).await;
}

/// A dry run against a file naming a runner that doesn't exist still fails —
/// validation is the point of the flag.
#[tokio::test]
async fn dry_run_still_reports_unresolvable_references() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_runner(server, ids.workspace, ids.runner).await;
	mount_deployment_list(server, ids.workspace, vec![]).await;

	let config = CONFIG.replace("test-runner", "runner-that-does-not-exist");
	let error = setup::apply(setup::state(ids.workspace), &config, &["--dry-run"])
		.await
		.expect_err("dry run should have failed on the missing runner");

	assert!(
		error.to_string().contains("runner-that-does-not-exist"),
		"the error should name the missing runner, got: {error}"
	);
}
