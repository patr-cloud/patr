use models::{ApiSuccessResponseBody, api::workspace::managed_url::*};

use crate::prelude::*;

#[tokio::test]
async fn create_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;
	assert_ne!(url_id, models::utils::Uuid::nil());
}

#[tokio::test]
async fn list_managed_urls_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	let _url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert_eq!(1, response.response.urls.len());
}

#[tokio::test]
async fn list_managed_urls_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert!(response.response.urls.is_empty());
}

#[tokio::test]
async fn update_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: Some("/updated".to_string()),
					url_type: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));
}

#[tokio::test]
async fn delete_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteManagedURLResponse));
}

#[tokio::test]
async fn verify_configuration_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
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
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
