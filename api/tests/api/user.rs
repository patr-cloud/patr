use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		user::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn get_user_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let info = setup
		.server
		.method(GetUserInfoRequest::METHOD, &GetUserInfoPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(user.username, info.response.basic_user_info.username);
	assert_eq!("Test", info.response.basic_user_info.first_name);
	assert_eq!("User", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn get_user_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.server
		.method(GetUserInfoRequest::METHOD, &GetUserInfoPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}

#[tokio::test]
async fn get_user_details_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let details = setup
		.server
		.method(
			GetUserDetailsRequest::METHOD,
			&GetUserDetailsPath {
				user_id: user.user_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetUserDetailsResponse>>();

	assert_eq!(user.username, details.response.basic_user_info.username);
}

#[tokio::test]
async fn get_user_details_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			GetUserDetailsRequest::METHOD,
			&GetUserDetailsPath {
				user_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent user"
	);
}

#[tokio::test]
async fn update_user_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	setup
		.server
		.method(
			UpdateUserInfoRequest::METHOD,
			&UpdateUserInfoPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateUserInfoRequest {
			first_name: Some("Updated".to_string()),
			last_name: Some("Name".to_string()),
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateUserInfoResponse));

	// Verify the update
	let info = setup
		.server
		.method(GetUserInfoRequest::METHOD, &GetUserInfoPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!("Updated", info.response.basic_user_info.first_name);
	assert_eq!("Name", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn update_user_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.server
		.method(
			UpdateUserInfoRequest::METHOD,
			&UpdateUserInfoPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&UpdateUserInfoRequest {
			first_name: Some("Hacker".to_string()),
			last_name: None,
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}

#[tokio::test]
async fn change_password_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let new_password = random_password();

	setup
		.server
		.method(
			ChangePasswordRequest::METHOD,
			&ChangePasswordPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&ChangePasswordRequest {
			current_password: user.password.clone(),
			new_password: new_password.clone(),
			mfa_otp: None,
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(ChangePasswordResponse));

	// Login with new password should work
	let (_token, _refresh) =
		login_test_user(&setup, &user.username, &new_password).await;
}

#[tokio::test]
async fn change_password_wrong_current() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ChangePasswordRequest::METHOD,
			&ChangePasswordPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&ChangePasswordRequest {
			current_password: "WrongCurrent@123".to_string(),
			new_password: random_password(),
			mfa_otp: None,
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong current password"
	);
}

#[tokio::test]
async fn list_workspaces_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListUserWorkspacesRequest::METHOD,
			&ListUserWorkspacesPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListUserWorkspacesRequest::METHOD,
			&ListUserWorkspacesPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert_eq!(1, response.response.workspaces.len());
	assert_eq!(ws.id, response.response.workspaces[0].id);
}

#[tokio::test]
async fn search_for_user_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let path = format!(
		"{}?query={}",
		SearchForUserPath.to_string(),
		user.username
	);
	let response = setup
		.server
		.method(SearchForUserRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<SearchForUserResponse>>();

	assert!(
		response
			.response
			.users
			.iter()
			.any(|u| u.username == user.username),
		"search should find the created user"
	);
}

#[tokio::test]
async fn search_for_user_no_results() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let path = format!(
		"{}?query=zzzznonexistentuserzzzzz",
		SearchForUserPath.to_string()
	);
	let response = setup
		.server
		.method(SearchForUserRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<SearchForUserResponse>>();

	assert!(
		response.response.users.is_empty(),
		"search for gibberish should return empty"
	);
}
