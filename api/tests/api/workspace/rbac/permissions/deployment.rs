use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::deployment::*,
	rbac::{DeploymentPermission, Permission},
};

use super::{all, exclude, include, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn deployment_view_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let view_id = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));

	let mut perms = BTreeMap::new();
	perms.insert(view_id, include(&[deployment.id]));
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with deployment::view should be able to get deployment info"
	);
}

#[tokio::test]
async fn deployment_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Deployment(DeploymentPermission::Create), all())],
	)
	.await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace_id)
		.await;

	let mt = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath { workspace_id })
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let mt_id = mt.response.machine_types[0].id;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id })
				.headers(CreateDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
					machine_type: mt_id,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
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
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn deployment_delete_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::Delete)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn deployment_view_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::Create)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b2 = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b2.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::view should be denied"
	);
}

#[tokio::test]
async fn deployment_create_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let mt = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: workspace.id,
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let mt_id = mt.response.machine_types[0].id;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
					machine_type: mt_id,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::create should be denied"
	);
}

#[tokio::test]
async fn deployment_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "dep1 should be accessible");

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dep2 should NOT be accessible"
	);
}

#[tokio::test]
async fn deployment_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment3 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		exclude(&[deployment2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"deployment1 should be accessible"
	);

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dep2 should be excluded"
	);

	let r3 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment3.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r3.status_code().is_success(), "dep3 should be accessible");
}

#[tokio::test]
async fn deployment_exclude_empty_grants_all() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success());

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r2.status_code().is_success());
}

#[tokio::test]
async fn deployment_exclude_empty_grants_list_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"listing deployments with Exclude([]) should succeed"
	);

	let body = response.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();
	let deployment_ids: Vec<_> = body.response.deployments.iter().map(|d| d.id).collect();

	assert!(
		deployment_ids.contains(&deployment1.id),
		"deployment1 should be in the list"
	);
	assert!(
		deployment_ids.contains(&deployment2.id),
		"deployment2 should be in the list"
	);
}

#[tokio::test]
async fn deployment_view_does_not_grant_edit() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_edit = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateDeploymentRequest>::builder()
				.path(UpdateDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(UpdateDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "registry.hub.docker.com".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: Uuid::nil(),
					machine_type: Uuid::nil(),
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
						environment_variables: BTreeMap::new(),
						startup_probe: None,
						liveness_probe: None,
						config_mounts: BTreeMap::new(),
						volumes: BTreeMap::new(),
					},
				})
				.build(),
		)
		.await;
	assert!(
		r_edit.status_code().is_client_error(),
		"view permission should not grant edit"
	);
}

#[tokio::test]
async fn deployment_create_does_not_grant_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Deployment(DeploymentPermission::Create), all())],
	)
	.await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace_id)
		.await;

	let mt_id = setup
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

	let created = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id })
				.headers(CreateDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
					machine_type: mt_id,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
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
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id,
					deployment_id: created.response.id.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"create-only member should not be able to view the deployment"
	);
}

#[tokio::test]
async fn deployment_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace_id)
		.await;
	let _dep = setup
		.create_test_deployment(&admin.access_token, workspace_id, runner.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.headers(ListDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();
	assert!(
		response.response.deployments.is_empty(),
		"a member without deployment View should see an empty list, not a 403"
	);
}

#[tokio::test]
async fn deployment_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let _dep = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should be denied access to the workspace's deployments"
	);
}

#[tokio::test]
async fn deployment_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}

#[tokio::test]
async fn deployment_stop_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<StopDeploymentRequest>::builder()
				.path(StopDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(StopDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::stop should be denied"
	);
}

#[tokio::test]
async fn deployment_include_multiple_resources() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let deployment3 = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment1.id, deployment2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "dep1 should be accessible");

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r2.status_code().is_success(), "dep2 should be accessible");

	let r3 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment3.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r3.status_code().is_client_error(),
		"dep3 should NOT be accessible"
	);
}

#[tokio::test]
async fn deployment_view_does_not_grant_start() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_start = setup
		.make_web_dashboard_call(
			ApiRequest::<StartDeploymentRequest>::builder()
				.path(StartDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(StartDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_start.status_code().is_client_error(),
		"view permission should not grant start"
	);
}

#[tokio::test]
async fn deployment_edit_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::Edit)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"edit permission should not grant delete"
	);
}

#[tokio::test]
async fn deployment_start_does_not_grant_stop() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::Start)),
		include(&[deployment.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<StopDeploymentRequest>::builder()
				.path(StopDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(StopDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"start permission should not grant stop"
	);
}
