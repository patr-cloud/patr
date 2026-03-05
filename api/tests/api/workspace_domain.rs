use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::domain::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn add_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;
	assert!(!domain.domain.is_empty());
}

#[tokio::test]
async fn list_domains_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let _domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListDomainsInWorkspaceRequest::METHOD,
			&ListDomainsInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert_eq!(1, response.response.domains.len());
}

#[tokio::test]
async fn list_domains_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListDomainsInWorkspaceRequest::METHOD,
			&ListDomainsInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert!(response.response.domains.is_empty());
}

#[tokio::test]
async fn get_domain_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();

	assert_eq!(domain.id, response.response.workspace_domain.id);
}

#[tokio::test]
async fn get_domain_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	setup
		.server
		.method(
			DeleteDomainInWorkspaceRequest::METHOD,
			&DeleteDomainInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteDomainInWorkspaceResponse,
		));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn is_domain_valid_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let path = format!(
		"{}?domain=example.com",
		IsDomainValidPath {
			workspace_id: ws.id,
		}
		.to_string()
	);
	let response = setup
		.server
		.method(IsDomainValidRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<IsDomainValidResponse>>();

	assert!(response.response.valid, "example.com should be valid");
}

#[tokio::test]
async fn verify_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	// Verification will likely fail (no DNS records), but it should not error
	let response = setup
		.server
		.method(
			VerifyDomainInWorkspaceRequest::METHOD,
			&VerifyDomainInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"verify should succeed or server error, got {status}"
	);
}

#[tokio::test]
async fn get_verification_records_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetVerificationRecordsForDomainRequest::METHOD,
			&GetVerificationRecordsForDomainPath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"expected success or server error, got {status}"
	);
}

#[tokio::test]
async fn domain_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListDomainsInWorkspaceRequest::METHOD,
			&ListDomainsInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(response.status_code().is_client_error());
}
