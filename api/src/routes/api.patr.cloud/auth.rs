use std::net::{IpAddr, Ipv4Addr};

use axum::{http::StatusCode, Router};
use models::{
	api::auth::{LoginRequest, LoginResponse},
	ErrorType,
};

use crate::{app::AppResponse, prelude::*};

pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		.mount_endpoint(login, state.clone())
}

async fn login<'a>(
	req: AppRequest<'a, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	let user_data = db::get_user_by_username_email_or_phone_number(
		req.database,
		user_id.to_lowercase().trim(),
	)
	.await?
    .ok_or(ErrorType::UserNotFound)?;

	let success = service::validate_hash(&password, &user_data.password)?;

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	let config = context.get_state().config.clone();
	let ip_address = routes::get_request_ip_address(&context);
	let user_agent = context.get_header("user-agent").unwrap_or_default();

	let (UserWebLogin { login_id, .. }, access_token, refresh_token) = service::sign_in_user(
		req.database,
		&user_data.id,
		&ip_address
			.parse()
			.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
		&user_agent,
		&config,
	)
	.await?;

	AppResponse::builder()
		.body(LoginResponse {
			access_token,
			refresh_token,
			login_id,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
