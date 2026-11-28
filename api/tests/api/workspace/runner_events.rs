use std::{collections::BTreeMap, time::Duration};

use api::redis_publisher::{self, DirtyResource};
use futures::StreamExt as _;
use models::{
	ApiSuccessResponseBody,
	api::workspace::{
		deployment::*,
		managed_url::*,
		runner::StreamRunnerDataForWorkspaceServerMsg::{self, *},
		secret::*,
	},
	rbac::ResourceType,
	utils::{BearerToken, StringifiedU16, Uuid},
};
use tokio_tungstenite::tungstenite::Message;

use super::runner::connect_runner;
use crate::prelude::*;

/// How long to wait for an event that should arrive.
const ARRIVES: Duration = Duration::from_secs(5);

/// How long to wait before deciding an event isn't coming.
const NEVER_ARRIVES: Duration = Duration::from_secs(1);

/// Read a deployment through the API.
async fn deployment_info(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> GetDeploymentInfoResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id,
					deployment_id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>()
		.response
}

/// Move a deployment to another runner, leaving everything else as it is.
async fn move_deployment(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
	runner: Uuid,
) {
	let info = deployment_info(setup, token, workspace_id, deployment_id).await;
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateDeploymentRequest>::builder()
				.path(UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.headers(UpdateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateDeploymentRequest {
					name: info.deployment.name.clone(),
					image_tag: info.deployment.image_tag.clone(),
					runner,
					machine_type: info.deployment.machine_type,
					running_details: info.running_details.clone(),
				})
				.build(),
		)
		.await
		.assert_status_success();
}

/// Create a deployment that exposes port 8080, which a managed URL can proxy
/// to, returning its ID.
async fn create_deployment_with_port(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	runner: Uuid,
) -> Uuid {
	let machine_type = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath { workspace_id })
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id;

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id })
				.headers(CreateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner,
					machine_type,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::from([(StringifiedU16::new(8080), ExposedPortType::Http)]),
						environment_variables: BTreeMap::new(),
						startup_probe: None,
						liveness_probe: None,
						config_mounts: BTreeMap::new(),
						volumes: BTreeMap::new(),
					},
					deploy_on_create: false,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response
		.id
		.id
}

/// Create a managed URL that proxies to a deployment, returning its ID and its
/// sub domain.
async fn create_proxy_url(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	domain_id: Uuid,
	deployment_id: Uuid,
) -> (Uuid, String) {
	let sub_domain = random_name(6);
	let id = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath { workspace_id })
				.headers(CreateManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: sub_domain.clone(),
					domain_id,
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id,
						port: 8080,
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
		.response
		.id
		.id;
	(id, sub_domain)
}

#[tokio::test]
async fn deployment_events_reach_its_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup.mark_test_runner_online(runner.id).await;
	let mut events = setup
		.subscribe_to_runner_events(workspace.id, runner.id)
		.await;

	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let created = |event: &StreamRunnerDataForWorkspaceServerMsg| {
		matches!(event, DeploymentUpdated { deployment: updated, .. } if
			updated.id == deployment.id && !updated.should_run &&
			updated.status == DeploymentStatus::Observed(ObservedDeploymentStatus::Stopped))
	};
	assert!(
		collect_runner_events(&mut events, ARRIVES, |events| events.iter().any(created))
			.await
			.iter()
			.any(created),
		"creating a deployment should send it to its runner"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<StartDeploymentRequest>::builder()
				.path(StartDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.query(StartDeploymentQuery {
					force_restart: false,
				})
				.headers(StartDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_success();
	let started = |event: &StreamRunnerDataForWorkspaceServerMsg| {
		matches!(event, DeploymentUpdated { deployment: updated, .. } if
			updated.id == deployment.id && updated.should_run &&
			updated.status == DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying))
	};
	assert!(
		collect_runner_events(&mut events, ARRIVES, |events| events.iter().any(started))
			.await
			.iter()
			.any(started),
		"starting a deployment should send its new state to its runner"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_success();
	let deleted = |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, DeploymentDeleted { id } if *id == deployment.id);
	assert!(
		collect_runner_events(&mut events, ARRIVES, |events| events.iter().any(deleted))
			.await
			.iter()
			.any(deleted),
		"deleting a deployment should tell its runner"
	);
}

/// A notification goes out only once its transaction commits, and never if it
/// rolls back.
#[tokio::test]
async fn notifications_are_published_only_after_commit() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup.mark_test_runner_online(runner.id).await;
	// Creating a secret tells no runner, so these are the only notifications
	// about them.
	let rolled_back = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let committed = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let mut events = setup
		.subscribe_to_runner_events(workspace.id, runner.id)
		.await;
	let about = |secret_id: Uuid| move |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, SecretUpdated { id, .. } if *id == secret_id);

	let mut transaction = setup.database().begin().await.unwrap();
	redis_publisher::mark_dirty(
		&mut transaction,
		DirtyResource {
			workspace_id: workspace.id,
			runner_id: runner.id,
			resource_type: ResourceType::Secret,
			resource_id: rolled_back.id,
		},
	)
	.await
	.unwrap();
	transaction.rollback().await.unwrap();

	let mut transaction = setup.database().begin().await.unwrap();
	redis_publisher::mark_dirty(
		&mut transaction,
		DirtyResource {
			workspace_id: workspace.id,
			runner_id: runner.id,
			resource_type: ResourceType::Secret,
			resource_id: committed.id,
		},
	)
	.await
	.unwrap();
	assert!(
		!collect_runner_events(&mut events, NEVER_ARRIVES, |events| {
			events.iter().any(about(committed.id))
		})
		.await
		.iter()
		.any(about(committed.id)),
		"nothing should be published while the transaction is open"
	);
	transaction.commit().await.unwrap();

	let published = collect_runner_events(&mut events, ARRIVES, |events| {
		events.iter().any(about(committed.id))
	})
	.await;
	assert!(
		published.iter().any(about(committed.id)),
		"a committed notification should be published"
	);
	assert!(
		!published
			.into_iter()
			.chain(collect_runner_events(&mut events, NEVER_ARRIVES, |_| false).await)
			.any(|event| about(rolled_back.id)(&event)),
		"a rolled back notification must never be published"
	);
}

/// After a move, the old runner drops the deployment and its managed URLs, the
/// new one picks them up, the ingress KV routes to the new runner, and the log
/// proxies stop caching the old one.
#[tokio::test]
async fn moving_a_deployment_tells_both_runners() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let old_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let new_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup.mark_test_runner_online(old_runner.id).await;
	setup.mark_test_runner_online(new_runner.id).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let deployment =
		create_deployment_with_port(&setup, &user.access_token, workspace.id, old_runner.id).await;
	let (managed_url, sub_domain) = create_proxy_url(
		&setup,
		&user.access_token,
		workspace.id,
		domain.id,
		deployment,
	)
	.await;
	let cache_key = api::redis::keys::runner_id_for_deployment(&deployment);
	setup
		.set_redis_value(&cache_key, &old_runner.id.to_string())
		.await;

	let mut old_events = setup
		.subscribe_to_runner_events(workspace.id, old_runner.id)
		.await;
	let mut new_events = setup
		.subscribe_to_runner_events(workspace.id, new_runner.id)
		.await;

	move_deployment(
		&setup,
		&user.access_token,
		workspace.id,
		deployment,
		new_runner.id,
	)
	.await;

	let deployment_dropped = |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, DeploymentDeleted { id } if *id == deployment);
	let url_dropped = |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, ManagedUrlDeleted { id } if *id == managed_url);
	let old = collect_runner_events(&mut old_events, ARRIVES, |events| {
		events.iter().any(deployment_dropped) && events.iter().any(url_dropped)
	})
	.await;
	assert!(
		old.iter().any(deployment_dropped),
		"the old runner should drop the deployment"
	);
	assert!(
		old.iter().any(url_dropped),
		"the old runner should drop the deployment's managed URL"
	);

	let deployment_added = |event: &StreamRunnerDataForWorkspaceServerMsg| {
		matches!(event, DeploymentUpdated { deployment: updated, .. } if
			updated.id == deployment && updated.runner == new_runner.id)
	};
	let url_added = |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, ManagedUrlUpdated { managed_url: updated } if updated.id == managed_url);
	let new = collect_runner_events(&mut new_events, ARRIVES, |events| {
		events.iter().any(deployment_added) && events.iter().any(url_added)
	})
	.await;
	assert!(
		new.iter().any(deployment_added),
		"the new runner should pick up the deployment"
	);
	assert!(
		new.iter().any(url_added),
		"the new runner should pick up the deployment's managed URL"
	);

	assert_eq!(
		setup
			.last_cloudflare_kv_write(&format!("{sub_domain}.{}", domain.domain))
			.await
			.expect("the managed URL's KV entry should be written")["/"]["runnerId"],
		new_runner.id.to_string(),
		"the ingress should route the managed URL to the new runner"
	);
	assert_eq!(
		setup.get_redis_value(&cache_key).await,
		None,
		"the cached runner for the deployment should be cleared"
	);
}

#[tokio::test]
async fn repointing_a_managed_url_tells_both_runners() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let old_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let new_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup.mark_test_runner_online(old_runner.id).await;
	setup.mark_test_runner_online(new_runner.id).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let old_deployment =
		create_deployment_with_port(&setup, &user.access_token, workspace.id, old_runner.id).await;
	let new_deployment =
		create_deployment_with_port(&setup, &user.access_token, workspace.id, new_runner.id).await;
	let (managed_url, _) = create_proxy_url(
		&setup,
		&user.access_token,
		workspace.id,
		domain.id,
		old_deployment,
	)
	.await;

	let mut old_events = setup
		.subscribe_to_runner_events(workspace.id, old_runner.id)
		.await;
	let mut new_events = setup
		.subscribe_to_runner_events(workspace.id, new_runner.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: managed_url,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id: new_deployment,
						port: 8080,
					},
				})
				.build(),
		)
		.await
		.assert_status_success();

	let dropped = |event: &StreamRunnerDataForWorkspaceServerMsg| matches!(event, ManagedUrlDeleted { id } if *id == managed_url);
	assert!(
		collect_runner_events(&mut old_events, ARRIVES, |events| events
			.iter()
			.any(dropped))
		.await
		.iter()
		.any(dropped),
		"the old runner should drop the managed URL"
	);
	let added = |event: &StreamRunnerDataForWorkspaceServerMsg| {
		matches!(event, ManagedUrlUpdated { managed_url: updated } if
		updated.id == managed_url &&
		updated.url_type == ManagedUrlType::ProxyDeployment {
			deployment_id: new_deployment,
			port: 8080,
		})
	};
	assert!(
		collect_runner_events(&mut new_events, ARRIVES, |events| events.iter().any(added))
			.await
			.iter()
			.any(added),
		"the new runner should pick up the managed URL"
	);
}

#[tokio::test]
async fn rotating_a_secret_tells_the_runners_using_it() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup.mark_test_runner_online(runner.id).await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	// Point an env var at the secret directly: `create_test_deployment` has no
	// way to pass environment variables.
	setup
		.execute_sql(&format!(
			"INSERT INTO deployment_environment_variable(deployment_id, workspace_id, name, value, \
			 secret_id) VALUES ('{}', '{}', 'API_KEY', NULL, '{}');",
			deployment.id, workspace.id, secret.id
		))
		.await;
	let mut events = setup
		.subscribe_to_runner_events(workspace.id, runner.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateSecretRequest>::builder()
				.path(UpdateSecretPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(UpdateSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateSecretRequest {
					name: secret.name.clone(),
					value: Some(random_name(16)),
				})
				.build(),
		)
		.await
		.assert_status_success();

	let last_updated = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>()
		.response
		.secret
		.last_updated;
	let rotated = |event: &StreamRunnerDataForWorkspaceServerMsg| {
		matches!(event, SecretUpdated { id, last_updated: updated } if
			*id == secret.id && *updated == last_updated)
	};
	assert!(
		collect_runner_events(&mut events, ARRIVES, |events| events.iter().any(rotated))
			.await
			.iter()
			.any(rotated),
		"rotating a secret should tell the runners using it"
	);
}

/// The runner's stream forwards what the bridge publishes to its channel.
#[tokio::test]
async fn runner_stream_forwards_its_events() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let mut runner_stream = connect_runner(&setup, workspace.id, runner.id, &runner.token).await;

	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let forwarded = tokio::time::timeout(ARRIVES, async {
		while let Some(message) = runner_stream.next().await {
			let Ok(Message::Text(text)) = message else {
				continue;
			};
			if let Ok(DeploymentUpdated {
				deployment: updated,
				..
			}) = serde_json::from_str::<StreamRunnerDataForWorkspaceServerMsg>(&text) &&
				updated.id == deployment.id
			{
				return true;
			}
		}
		false
	})
	.await
	.unwrap_or(false);
	assert!(
		forwarded,
		"the runner should get the deployment over its stream"
	);
	runner_stream.close(None).await.unwrap();
}
