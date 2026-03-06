use std::collections::BTreeMap;

use api::routes::registry_patr_cloud::handlers::GetApiVersionPath;

use crate::prelude::*;

#[tokio::test]
async fn version_check_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetApiVersionPath> {
			path: GetApiVersionPath,
			query: (),
			headers: api::routes::registry_patr_cloud::handlers::GetApiVersionRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), http::StatusCode::OK);
}
