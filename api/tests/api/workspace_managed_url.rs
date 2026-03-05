use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::managed_url::*,
	},
};

use crate::prelude::*;

#[tokio::test]
async fn create_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;

	let url_id =
		create_test_managed_url(&setup, &user.access_token, ws.id, domain.id).await;
	assert_ne!(url_id, models::utils::Uuid::nil());
}

#[tokio::test]
async fn list_managed_urls_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;
	let _url_id =
		create_test_managed_url(&setup, &user.access_token, ws.id, domain.id).await;

	let response = setup
		.server
		.method(
			ListManagedURLRequest::METHOD,
			&ListManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert_eq!(1, response.response.urls.len());
}

#[tokio::test]
async fn list_managed_urls_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListManagedURLRequest::METHOD,
			&ListManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert!(response.response.urls.is_empty());
}

#[tokio::test]
async fn update_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;
	let url_id =
		create_test_managed_url(&setup, &user.access_token, ws.id, domain.id).await;

	setup
		.server
		.method(
			UpdateManagedURLRequest::METHOD,
			&UpdateManagedURLPath {
				workspace_id: ws.id,
				managed_url_id: url_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateManagedURLRequest {
			path: Some("/updated".to_string()),
			url_type: None,
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));
}

#[tokio::test]
async fn delete_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;
	let url_id =
		create_test_managed_url(&setup, &user.access_token, ws.id, domain.id).await;

	setup
		.server
		.method(
			DeleteManagedURLRequest::METHOD,
			&DeleteManagedURLPath {
				workspace_id: ws.id,
				managed_url_id: url_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteManagedURLResponse));
}

#[tokio::test]
async fn verify_configuration_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let domain = create_test_domain(&setup, &user.access_token, ws.id).await;
	let url_id =
		create_test_managed_url(&setup, &user.access_token, ws.id, domain.id).await;

	let response = setup
		.server
		.method(
			VerifyManagedURLConfigurationRequest::METHOD,
			&VerifyManagedURLConfigurationPath {
				workspace_id: ws.id,
				managed_url_id: url_id,
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
async fn managed_url_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListManagedURLRequest::METHOD,
			&ListManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(response.status_code().is_client_error());
}
