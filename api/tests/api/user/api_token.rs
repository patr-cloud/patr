use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	str::FromStr,
};

use ipnetwork::IpNetwork;
use models::{
	ApiSuccessResponseBody,
	api::{
		user::*,
		workspace::deployment::{
			CreateDeploymentPath,
			CreateDeploymentRequest,
			CreateDeploymentRequestHeaders,
			DeploymentRegistry,
			DeploymentRunningDetails,
			GetDeploymentInfoPath,
			GetDeploymentInfoRequest,
			GetDeploymentInfoRequestHeaders,
		},
	},
	rbac::{DeploymentPermission, Permission, ResourcePermissionType, WorkspacePermission},
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

#[tokio::test]
async fn create_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let api_token = setup
		.create_test_api_token(&user.access_token, {
			let mut map = BTreeMap::new();

			map.insert(workspace.id, WorkspacePermission::SuperAdmin);

			map
		})
		.await;
	assert!(!api_token.token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn list_api_tokens_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let perms = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let _t1 = setup
		.create_test_api_token(&user.access_token, perms.clone())
		.await;
	let _t2 = setup.create_test_api_token(&user.access_token, perms).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();

	assert!(
		response.response.tokens.len() >= 2,
		"should have at least 2 tokens"
	);
}

#[tokio::test]
async fn get_api_token_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(api_token.name, response.response.token.name);
}

#[tokio::test]
async fn update_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let new_name = random_name(8);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					name: Some(new_name.clone()),
					permissions: None,
					token_nbf: None,
					token_exp: None,
					allowed_ips: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateApiTokenResponse));

	// Verify the update
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(new_name, response.response.token.name);
}

#[tokio::test]
async fn revoke_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"revoked token should not be found"
	);
}

#[tokio::test]
async fn regenerate_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateApiTokenRequest>::builder()
				.path(RegenerateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RegenerateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateApiTokenResponse>>();

	assert_ne!(
		api_token.token, response.response.token,
		"regenerated token should be different"
	);
}

#[tokio::test]
async fn get_api_token_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: Uuid::nil(),
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent token"
	);
}

#[tokio::test]
async fn create_api_token_with_empty_permissions_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"creating an API token with empty permissions should fail, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.headers(ListApiTokensRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}

#[tokio::test]
async fn create_api_token_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let perms = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let first = setup
		.create_test_api_token(&user.access_token, perms.clone())
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: first.name.clone(),
						permissions: perms,
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate token name, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn update_api_token_name_conflict() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let perms = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let first = setup
		.create_test_api_token(&user.access_token, perms.clone())
		.await;
	let second = setup.create_test_api_token(&user.access_token, perms).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: second.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					name: Some(first.name.clone()),
					permissions: None,
					token_nbf: None,
					token_exp: None,
					allowed_ips: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"renaming a token to a name already in use should fail, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn use_api_token_for_auth() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	// Use the API token to list workspaces — should succeed since it auths as
	// the owning user.
	let response = setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert!(
		response
			.response
			.workspaces
			.iter()
			.any(|w| w.id == workspace.id),
		"workspace should be visible via API token"
	);
}

#[tokio::test]
async fn use_revoked_api_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();
	let response = setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"revoked API token should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn list_api_tokens_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let perms = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	for _ in 0..3 {
		setup
			.create_test_api_token(&user.access_token, perms.clone())
			.await;
	}

	let page0 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 0,
					additional_query: (),
				})
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();
	assert_eq!(
		page0.response.tokens.len(),
		2,
		"page 0 should have 2 tokens"
	);

	let page1 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 1,
					additional_query: (),
				})
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();
	assert!(
		page1.response.tokens.len() >= 1,
		"page 1 should have remaining token(s)"
	);

	// Pages must not overlap.
	let page0_ids: BTreeSet<Uuid> = page0.response.tokens.iter().map(|t| t.id).collect();
	let page1_ids: BTreeSet<Uuid> = page1.response.tokens.iter().map(|t| t.id).collect();
	assert!(
		page0_ids.is_disjoint(&page1_ids),
		"pages should not contain overlapping tokens"
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_allows_listed_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			allowed,
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"request from allowed IP should succeed, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_blocks_unlisted_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let blocked: IpAddr = "5.6.7.8".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			blocked,
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"request from disallowed IP should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_scoped_permissions_allows_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let _deployment2 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let token_perms = BTreeMap::from([(
		workspace.id,
		WorkspacePermission::Member {
			permissions: BTreeMap::from([(
				view_perm,
				ResourcePermissionType::Include(BTreeSet::from([deployment1.id])),
			)]),
		},
	)]);
	let api_token = setup
		.create_test_api_token(&user.access_token, token_perms)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"included deployment should be readable, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_scoped_permissions_denies_other_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let token_perms = BTreeMap::from([(
		workspace.id,
		WorkspacePermission::Member {
			permissions: BTreeMap::from([(
				view_perm,
				ResourcePermissionType::Include(BTreeSet::from([deployment1.id])),
			)]),
		},
	)]);
	let api_token = setup
		.create_test_api_token(&user.access_token, token_perms)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-included deployment should be denied, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_view_permission_denies_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let token_perms = BTreeMap::from([(
		workspace.id,
		WorkspacePermission::Member {
			permissions: BTreeMap::from([(
				view_perm,
				ResourcePermissionType::Exclude(BTreeSet::new()), // grants View on all
			)]),
		},
	)]);
	let api_token = setup
		.create_test_api_token(&user.access_token, token_perms)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	// Pick any machine type to satisfy the request body schema.
	use models::api::workspace::deployment::{
		ListAllDeploymentMachineTypePath,
		ListAllDeploymentMachineTypeRequest,
		ListAllDeploymentMachineTypeRequestHeaders,
		ListAllDeploymentMachineTypeResponse,
	};
	let machine_types = setup
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
	let machine_type_id = machine_types.response.machine_types[0].id;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: token_bearer,
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
					machine_type: machine_type_id,
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
		"View-only token should be denied Create, got {}",
		response.status_code()
	);
}
