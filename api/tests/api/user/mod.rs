use models::{ApiSuccessResponseBody, api::user::*, utils::Uuid};

use crate::prelude::*;

pub mod api_token;
pub mod mfa;
pub mod social_login;

#[tokio::test]
async fn get_user_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(user.email, info.response.email);
	assert_eq!("Test", info.response.basic_user_info.first_name);
	assert_eq!("User", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn get_user_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
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
async fn get_user_details_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let details = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserDetailsRequest>::builder()
				.path(GetUserDetailsPath {
					user_id: user.user_id,
				})
				.headers(GetUserDetailsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserDetailsResponse>>();

	assert_eq!("Test", details.response.basic_user_info.first_name);
}

#[tokio::test]
async fn get_user_details_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserDetailsRequest>::builder()
				.path(GetUserDetailsPath {
					user_id: Uuid::nil(),
				})
				.headers(GetUserDetailsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent user"
	);
}

#[tokio::test]
async fn update_user_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserInfoRequest>::builder()
				.headers(UpdateUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserInfoRequest {
					first_name: Some("Updated".to_string()),
					last_name: Some("Name".to_string()),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateUserInfoResponse));

	// Verify the update
	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!("Updated", info.response.basic_user_info.first_name);
	assert_eq!("Name", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn update_user_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserInfoRequest>::builder()
				.headers(UpdateUserInfoRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserInfoRequest {
					first_name: Some("Hacker".to_string()),
					last_name: None,
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
async fn change_password_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let new_password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<ChangePasswordRequest>::builder()
				.headers(ChangePasswordRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ChangePasswordRequest {
					current_password: user.password.clone(),
					new_password: new_password.clone(),
					mfa_otp: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ChangePasswordResponse));

	// Login with new password should work
	let (_token, _refresh) = setup.login_test_user(&user.email, &new_password).await;
}

#[tokio::test]
async fn change_password_wrong_current() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ChangePasswordRequest>::builder()
				.headers(ChangePasswordRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ChangePasswordRequest {
					current_password: "WrongCurrent@123".to_string(),
					new_password: random_password(),
					mfa_otp: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong current password"
	);
}

#[tokio::test]
async fn list_workspaces_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert!(
		response.response.workspaces.is_empty(),
		"new user should have no workspaces"
	);
}

#[tokio::test]
async fn list_workspaces_after_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert_eq!(1, response.response.workspaces.len());
	assert_eq!(workspace.id, response.response.workspaces[0].id);
}

#[tokio::test]
async fn update_user_info_first_name_persists() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserInfoRequest>::builder()
				.headers(UpdateUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserInfoRequest {
					first_name: Some("Alice".to_string()),
					last_name: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateUserInfoResponse));

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!("Alice", info.response.basic_user_info.first_name);
	// Last name unchanged.
	assert_eq!("User", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn update_user_info_empty_fields() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserInfoRequest>::builder()
				.headers(UpdateUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserInfoRequest {
					first_name: None,
					last_name: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateUserInfoResponse));

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!("Test", info.response.basic_user_info.first_name);
	assert_eq!("User", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn list_workspaces_multiple() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ws1 = setup.create_test_workspace(&user.access_token).await;
	let ws2 = setup.create_test_workspace(&user.access_token).await;
	let ws3 = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert_eq!(3, response.response.workspaces.len());
	let ids: Vec<_> = response.response.workspaces.iter().map(|w| w.id).collect();
	assert!(ids.contains(&ws1.id));
	assert!(ids.contains(&ws2.id));
	assert!(ids.contains(&ws3.id));
}

#[tokio::test]
async fn get_user_details_own_id() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let details = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserDetailsRequest>::builder()
				.path(GetUserDetailsPath {
					user_id: user.user_id,
				})
				.headers(GetUserDetailsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserDetailsResponse>>();

	assert_eq!(user.user_id, details.response.basic_user_info.id);
}

#[tokio::test]
async fn change_password_same_as_current() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ChangePasswordRequest>::builder()
				.headers(ChangePasswordRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ChangePasswordRequest {
					current_password: user.password.clone(),
					new_password: user.password.clone(),
					mfa_otp: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"changing password to the same value should fail, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn change_password_new_invalid() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ChangePasswordRequest>::builder()
				.headers(ChangePasswordRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ChangePasswordRequest {
					current_password: user.password.clone(),
					new_password: "short".to_string(),
					mfa_otp: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for weak new password"
	);
}
