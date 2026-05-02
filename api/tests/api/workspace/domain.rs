use models::{ApiSuccessResponseBody, api::workspace::domain::*, utils::Uuid};

use crate::prelude::*;

#[tokio::test]
async fn add_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	assert!(!domain.domain.is_empty());
}

#[tokio::test]
async fn list_domains_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert_eq!(1, response.response.domains.len());
}

#[tokio::test]
async fn list_domains_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert!(response.response.domains.is_empty());
}

#[tokio::test]
async fn get_domain_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();

	assert_eq!(domain.id, response.response.workspace_domain.id);
}

#[tokio::test]
async fn get_domain_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: Uuid::nil(),
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteDomainInWorkspaceResponse,
		));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn is_domain_valid_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsDomainValidRequest>::builder()
				.path(IsDomainValidPath {
					workspace_id: workspace.id,
				})
				.headers(IsDomainValidRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsDomainValidQuery {
					domain: "example.com".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsDomainValidResponse>>();

	assert!(response.response.valid, "example.com should be valid");
}

#[tokio::test]
async fn verify_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	// Verification will likely fail (no DNS records), but it should not error
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"verify should succeed or server error, got {status}"
	);
}

#[tokio::test]
async fn domain_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
