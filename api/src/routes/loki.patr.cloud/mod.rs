use std::{
	net::{IpAddr, Ipv4Addr},
	str::FromStr,
};

use api_models::utils::Uuid;
use base64::prelude::*;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use itertools::Itertools;
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};

use super::get_request_ip_address;
use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{rbac::permissions, ApiTokenData, UserAuthenticationData},
	pin_fn,
	utils::{Error, ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/loki/api/v1/push",
		[EveMiddleware::CustomFunction(pin_fn!(push_loki_logs))],
	);

	sub_app
}

async fn push_loki_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let (region_id, api_token) = context
		.get_header(header::AUTHORIZATION.as_str())
		.and_then(|auth_header| {
			auth_header
				.strip_prefix("Basic ")
				.map(|encoded_auth| encoded_auth.to_owned())
		})
		.and_then(|encoded_auth| BASE64_STANDARD.decode(encoded_auth).ok())
		.and_then(|decoded_auth| String::from_utf8(decoded_auth).ok())
		.and_then(|decoded_auth_str| {
			decoded_auth_str
				.split_once(':')
				.and_then(|(username, password)| {
					Some((Uuid::parse_str(username).ok()?, password.to_owned()))
				})
		})
		.status(401)
		.body(error!(UNAUTHORIZED).to_string())?;

	let region =
		db::get_resource_by_id(context.get_database_connection(), &region_id)
			.await?
			.status(404)
			.body(error!(NOT_FOUND).to_string())?;

	let from_ip_addr = get_request_ip_address(&context)
		.parse()
		.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

	let mut redis_conn = context.get_redis_connection().clone();

	let api_token_data = ApiTokenData::decode(
		context.get_database_connection(),
		&mut redis_conn,
		&api_token,
		&from_ip_addr,
	)
	.await?;

	let has_permission = UserAuthenticationData::ApiToken(api_token_data)
		.has_access_for_requested_action(
			&region.owner_id,
			&region.id,
			permissions::workspace::region::LOGS_PUSH,
		);

	if !has_permission {
		return Error::as_result()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	let config = &context.get_state().config;

	let mut request_headers = context
		.get_request()
		.get_headers()
		.iter()
		.filter_map(|(name, value)| match name.as_str() {
			"host" => None,
			_ => {
				let name = HeaderName::from_str(name).ok()?;
				let value = value.iter().cloned().join(", ");
				let value = HeaderValue::from_str(&value).ok()?;
				Some((name, value))
			}
		})
		.collect::<HeaderMap>();
	request_headers.insert(
		"X-Scope-OrgID",
		HeaderValue::from_str(&region.owner_id.to_string())
			.expect("workpsace_id to headervalue should not panic"),
	);

	let response = reqwest::Client::new()
		.post(format!("https://{}/loki/api/v1/push", config.loki.host))
		.basic_auth(&config.loki.username, Some(&config.loki.password))
		.headers(request_headers)
		.body(context.get_request().get_body_bytes().to_owned())
		.send()
		.await?;

	context.status(response.status().as_u16());
	for (name, value) in response.headers() {
		context.header(name.as_str(), value.to_str().status(500)?);
	}
	context.body_bytes(&response.bytes().await.status(500)?);

	Ok(context)
}
