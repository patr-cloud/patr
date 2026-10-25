use api::redis::keys;
use models::{
	ApiSuccessResponseBody,
	api::{
		user::{UpdateUserInfoRequest, UpdateUserInfoRequestHeaders, UpdateUserInfoResponse},
		workspace::{
			GetWorkspaceInfoPath,
			GetWorkspaceInfoRequest,
			GetWorkspaceInfoRequestHeaders,
			rbac::user::RoleBindingGrant,
			service_account::*,
		},
	},
	rbac::{Permission, ServiceAccountPermission},
	utils::{ListResourceQuery, Uuid},
};
use sha2::{Digest as _, Sha256};

use crate::prelude::*;

// ── Create ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_service_account_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	assert!(!sa.name.is_empty());
	assert!(
		sa.token.starts_with("patr_sa_"),
		"token should start with patr_sa_"
	);
}

/// How many emails of `email_type` about the service account named
/// `service_account_name` are queued for `to`.
async fn queued_service_account_emails(
	setup: &TestSetup,
	email_type: &str,
	to: &str,
	service_account_name: &str,
) -> i64 {
	sqlx::query_scalar::<_, i64>(
		r#"
		SELECT
			COUNT(*)
		FROM
			apalis.jobs
		WHERE
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,type}' = $1 AND
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,to}' = $2 AND
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,data,serviceAccountName}' = $3;
		"#,
	)
	.bind(email_type)
	.bind(to)
	.bind(service_account_name)
	.fetch_one(setup.database())
	.await
	.expect("queued email query")
}

/// Who each queued email of `email_type` about the service account named
/// `service_account_name` is to, paired with who its `actor_field` names as
/// having acted.
pub async fn queued_service_account_email_actors(
	setup: &TestSetup,
	email_type: &str,
	actor_field: &str,
	service_account_name: &str,
) -> Vec<(String, String)> {
	sqlx::query_as::<_, (String, String)>(
		r#"
		SELECT
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,to}',
			CONVERT_FROM(job, 'UTF8')::JSONB #>> ARRAY['data', 'data', $2]
		FROM
			apalis.jobs
		WHERE
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,type}' = $1 AND
			CONVERT_FROM(job, 'UTF8')::JSONB #>> '{data,data,serviceAccountName}' = $3;
		"#,
	)
	.bind(email_type)
	.bind(actor_field)
	.bind(service_account_name)
	.fetch_all(setup.database())
	.await
	.expect("queued email query")
}

/// Add a member holding `permission` across the workspace, renamed to `Acting
/// Member` so an email can tell them apart from the owner, and return the
/// token they renamed themselves with.
async fn add_renamed_member(
	setup: &TestSetup,
	owner: &TestUser,
	workspace_id: Uuid,
	permission: Permission,
) -> BearerToken {
	let role = setup
		.create_role_with_permissions(
			&owner.access_token,
			workspace_id,
			vec![setup.get_permission_id(permission)],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace_id, role.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserInfoRequest>::builder()
				.headers(UpdateUserInfoRequestHeaders {
					authorization: member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserInfoRequest {
					first_name: "Acting".to_string(),
					last_name: "Member".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateUserInfoResponse));

	member.access_token
}

#[tokio::test]
async fn create_service_account_emails_the_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	assert_eq!(
		1,
		queued_service_account_emails(&setup, "serviceAccountCreated", &user.email, &sa.name).await,
		"creating a service account should email the workspace's super admin"
	);
}

/// A member creating a service account emails the super admin, not the
/// member, and names the member as who created it.
#[tokio::test]
async fn create_service_account_by_a_member_emails_the_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let member_token = add_renamed_member(
		&setup,
		&owner,
		workspace.id,
		Permission::ServiceAccount(ServiceAccountPermission::Create),
	)
	.await;

	let sa = setup
		.create_test_service_account(&member_token, workspace.id, vec![])
		.await;

	assert_eq!(
		vec![(owner.email, "Acting Member".to_string())],
		queued_service_account_email_actors(&setup, "serviceAccountCreated", "createdBy", &sa.name)
			.await,
		"a member creating a service account should email only the super admin, naming the member"
	);
}

#[tokio::test]
async fn create_service_account_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name,
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate service account name"
	);
}

#[tokio::test]
async fn create_service_account_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: "!!!".to_string(),
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid service account name"
	);
}

/// Names share `RESOURCE_NAME_REGEX` (2–255 characters), so the column has to
/// hold the full 255 rather than truncating at some narrower width.
#[tokio::test]
async fn create_service_account_name_length_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	for (length, expect_ok) in [(101, true), (255, true), (256, false)] {
		// `random_name(8)` is nine characters; pad it out to the length under test.
		let name = format!("{}{}", random_name(8), "a".repeat(length - 9));
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CreateServiceAccountRequest>::builder()
					.path(CreateServiceAccountPath {
						workspace_id: workspace.id,
					})
					.headers(CreateServiceAccountRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateServiceAccountRequest {
						service_account: ServiceAccount {
							name,
							description: None,
						},
						role_bindings: vec![],
					})
					.build(),
			)
			.await;

		if expect_ok {
			assert!(
				response.status_code().is_success(),
				"a {length}-char service account name should be accepted, got {}",
				response.status_code()
			);
		} else {
			assert!(
				response.status_code().is_client_error(),
				"a {length}-char service account name should be rejected, got {}",
				response.status_code()
			);
		}
	}
}

/// Descriptions share `ROLE_DESCRIPTION_REGEX`: at most 500 characters and no
/// HTML metacharacters.
#[tokio::test]
async fn create_service_account_description_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	for (description, expect_ok) in [
		("a".repeat(500), true),
		("a".repeat(501), false),
		("runs <b>deploys</b>".to_string(), false),
	] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CreateServiceAccountRequest>::builder()
					.path(CreateServiceAccountPath {
						workspace_id: workspace.id,
					})
					.headers(CreateServiceAccountRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateServiceAccountRequest {
						service_account: ServiceAccount {
							name: random_name(8),
							description: Some(description.clone()),
						},
						role_bindings: vec![],
					})
					.build(),
			)
			.await;

		if expect_ok {
			assert!(
				response.status_code().is_success(),
				"description {description:?} should be accepted, got {}",
				response.status_code()
			);
		} else {
			assert!(
				response.status_code().is_client_error(),
				"description {description:?} should be rejected, got {}",
				response.status_code()
			);
		}
	}
}

/// A role from another workspace can't be granted, and the failed create
/// leaves no half-made service account behind.
#[tokio::test]
async fn create_service_account_cross_workspace_role() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let role_b = setup
		.create_test_role(&user.access_token, workspace_b.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace_a.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: vec![RoleBindingGrant {
						role_id: role_b.id,
						resource_id: workspace_a.id,
					}],
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"granting a role from another workspace should be rejected, got {}",
		response.status_code()
	);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace_a.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();
	assert!(response.response.service_accounts.is_empty());
}

/// A grant can't be scoped to a resource in another workspace.
#[tokio::test]
async fn create_service_account_cross_workspace_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let role_a = setup
		.create_test_role(&user.access_token, workspace_a.id)
		.await;
	let runner_b = setup
		.create_test_runner(&user.access_token, workspace_b.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace_a.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: vec![RoleBindingGrant {
						role_id: role_a.id,
						resource_id: runner_b.id,
					}],
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"scoping a grant to another workspace's resource should be rejected, got {}",
		response.status_code()
	);
}

// ── List ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_service_accounts_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();

	assert_eq!(1, response.response.service_accounts.len());
}

#[tokio::test]
async fn list_service_accounts_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();

	assert!(response.response.service_accounts.is_empty());
}

/// The list is ordered created descending (newest first).
#[tokio::test]
async fn list_service_accounts_ordered_created_desc() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let mut names = Vec::new();
	for _ in 0..3 {
		let sa = setup
			.create_test_service_account(&user.access_token, workspace.id, vec![])
			.await;
		names.push(sa.name);
	}

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();

	let listed = response
		.response
		.service_accounts
		.iter()
		.map(|sa| sa.name.clone())
		.collect::<Vec<_>>();
	names.reverse();
	assert_eq!(
		names, listed,
		"service accounts should be ordered created DESC"
	);
}

/// page/count slice the service account list and pages don't overlap.
#[tokio::test]
async fn list_service_accounts_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	for _ in 0..5 {
		setup
			.create_test_service_account(&user.access_token, workspace.id, vec![])
			.await;
	}

	let mut pages = Vec::new();
	for page in 0..2usize {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<ListServiceAccountsRequest>::builder()
					.path(ListServiceAccountsPath {
						workspace_id: workspace.id,
					})
					.query(ListResourceQuery {
						sort: None,
						search: Default::default(),
						count: 2,
						page,
						additional_query: (),
					})
					.headers(ListServiceAccountsRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await;
		assert_eq!("5", response.header("x-total-count"));
		pages.push(response.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>());
	}
	assert_eq!(2, pages[0].response.service_accounts.len());
	assert_eq!(2, pages[1].response.service_accounts.len());

	let ids = pages[0]
		.response
		.service_accounts
		.iter()
		.chain(pages[1].response.service_accounts.iter())
		.map(|sa| sa.id)
		.collect::<std::collections::BTreeSet<_>>();
	assert_eq!(4, ids.len(), "the two pages should not overlap");
}

/// A non-zero page past the end of the result set is rejected as out of bounds
/// (PageOutOfBounds → 400) rather than returning an empty page.
#[tokio::test]
async fn list_service_accounts_page_out_of_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 10,
					page: 50,
					additional_query: (),
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a page past the end should be PageOutOfBounds (400)"
	);
}

/// The total count only covers the service accounts that match the search,
/// whether it's by name or by description.
#[tokio::test]
async fn list_service_accounts_search_counts_only_matches() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let by_name = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let description = random_name(12);
	let by_description = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: Some(format!("deploys {description} nightly")),
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateServiceAccountResponse>>()
		.response
		.id
		.id;
	setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	for (search, expected_id) in [
		(
			ServiceAccountSearchParams {
				name: Some(by_name.name.clone()),
				..Default::default()
			},
			by_name.id,
		),
		(
			ServiceAccountSearchParams {
				description: Some(description.clone()),
				..Default::default()
			},
			by_description,
		),
	] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<ListServiceAccountsRequest>::builder()
					.path(ListServiceAccountsPath {
						workspace_id: workspace.id,
					})
					.query(ListResourceQuery {
						sort: None,
						search,
						count: 10,
						page: 0,
						additional_query: (),
					})
					.headers(ListServiceAccountsRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await;

		assert_eq!("1", response.header("x-total-count"));
		let body = response.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();
		assert_eq!(1, body.response.service_accounts.len());
		assert_eq!(expected_id, body.response.service_accounts[0].id);
	}
}

// ── Get ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_service_account_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(sa.name, response.response.service_account.name);
}

#[tokio::test]
async fn get_service_account_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: Uuid::nil(),
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent service account"
	);
}

// ── Update ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_service_account_name_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let new_name = random_name(8);
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: new_name.clone(),
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(new_name, response.response.service_account.name);
}

#[tokio::test]
async fn update_service_account_role_bindings_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let role = setup
		.create_role_with_permissions(
			&user.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	// The whole workspace: a grant at the root covers everything under it.
	let grant = RoleBindingGrant {
		role_id: role.id,
		resource_id: workspace.id,
	};

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
					},
					role_bindings: vec![grant.clone()],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify the grants came back
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(vec![grant], response.response.role_bindings);
}

#[tokio::test]
async fn update_service_account_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let taken = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: taken.name,
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_service_account_of_another_type_is_not_found() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: runner.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

/// A role from another workspace can't be granted on update, and the failed
/// update leaves the existing grants in place.
#[tokio::test]
async fn update_service_account_cross_workspace_role() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let role_a = setup
		.create_test_role(&user.access_token, workspace_a.id)
		.await;
	let role_b = setup
		.create_test_role(&user.access_token, workspace_b.id)
		.await;
	let grant = RoleBindingGrant {
		role_id: role_a.id,
		resource_id: workspace_a.id,
	};
	let sa = setup
		.create_test_service_account(&user.access_token, workspace_a.id, vec![grant.clone()])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace_a.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
					},
					role_bindings: vec![RoleBindingGrant {
						role_id: role_b.id,
						resource_id: workspace_a.id,
					}],
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"granting a role from another workspace should be rejected, got {}",
		response.status_code()
	);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace_a.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();
	assert_eq!(vec![grant], response.response.role_bindings);
}

/// A grant can't be scoped to a resource in another workspace on update.
#[tokio::test]
async fn update_service_account_cross_workspace_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let role_a = setup
		.create_test_role(&user.access_token, workspace_a.id)
		.await;
	let runner_b = setup
		.create_test_runner(&user.access_token, workspace_b.id)
		.await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace_a.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace_a.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
					},
					role_bindings: vec![RoleBindingGrant {
						role_id: role_a.id,
						resource_id: runner_b.id,
					}],
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"scoping a grant to another workspace's resource should be rejected, got {}",
		response.status_code()
	);
}

// ── Delete ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_service_account_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteServiceAccountResponse));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted service account should not be found"
	);
}

#[tokio::test]
async fn delete_service_account_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: Uuid::nil(),
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent service account"
	);
}

// ── Token Regeneration ──────────────────────────────────────────────────

#[tokio::test]
async fn regenerate_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateServiceAccountTokenResponse>>();

	let new_token = &response.response.token;
	assert!(
		new_token.starts_with("patr_sa_"),
		"new token should start with patr_sa_"
	);
	assert_ne!(
		&sa.token, new_token,
		"new token should differ from original"
	);
}

#[tokio::test]
async fn regenerate_token_emails_the_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::ACCEPTED, response.status_code());

	assert_eq!(
		1,
		queued_service_account_emails(
			&setup,
			"serviceAccountTokenRegenerated",
			&user.email,
			&sa.name
		)
		.await,
		"regenerating a service account's token should email the workspace's super admin"
	);
}

/// A member regenerating a service account's token emails the super admin,
/// not the member, and names the member as who regenerated it.
#[tokio::test]
async fn regenerate_token_by_a_member_emails_the_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let sa = setup
		.create_test_service_account(&owner.access_token, workspace.id, vec![])
		.await;
	let member_token = add_renamed_member(
		&setup,
		&owner,
		workspace.id,
		Permission::ServiceAccount(ServiceAccountPermission::RegenerateToken),
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: member_token,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::ACCEPTED, response.status_code());

	assert_eq!(
		vec![(owner.email, "Acting Member".to_string())],
		queued_service_account_email_actors(
			&setup,
			"serviceAccountTokenRegenerated",
			"regeneratedBy",
			&sa.name
		)
		.await,
		"a member regenerating a service account's token should email only the super admin, \
		 naming the member"
	);
}

/// A regenerated service account's old token stops working at once, even
/// once cached, and its cached entry is dropped.
#[tokio::test]
async fn regenerated_service_account_old_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let get_workspace_info = |token: &str| {
		setup.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert!(
		get_workspace_info(&sa.token)
			.await
			.status_code()
			.is_success(),
		"the token should work before it is regenerated"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&sa.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);

	let new_token = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateServiceAccountTokenResponse>>()
		.response
		.token;

	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"regenerating should drop the old token's cached entry"
	);
	assert_eq!(
		401,
		get_workspace_info(&sa.token).await.status_code().as_u16(),
		"the old token should be rejected with 401"
	);
	assert!(
		get_workspace_info(&new_token)
			.await
			.status_code()
			.is_success(),
		"the regenerated token should work"
	);
}

/// A deleted service account's token stops working at once, even once cached,
/// and its cached entry is dropped.
#[tokio::test]
async fn deleted_service_account_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let get_workspace_info = || {
		setup.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&sa.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert!(
		get_workspace_info().await.status_code().is_success(),
		"the token should work before the account is deleted"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&sa.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);

	assert!(
		setup
			.make_web_dashboard_call(
				ApiRequest::<DeleteServiceAccountRequest>::builder()
					.path(DeleteServiceAccountPath {
						workspace_id: workspace.id,
						service_account_id: sa.id,
					})
					.headers(DeleteServiceAccountRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code()
			.is_success(),
		"deleting the account should succeed"
	);

	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"deleting should drop the cached entry"
	);
	assert_eq!(
		401,
		get_workspace_info().await.status_code().as_u16(),
		"a deleted account's token should be rejected with 401"
	);
}

// ── Unauthorized ────────────────────────────────────────────────────────

#[tokio::test]
async fn service_account_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
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
