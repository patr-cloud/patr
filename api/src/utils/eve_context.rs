use std::{
	fmt::{Debug, Formatter},
	net::IpAddr,
	str::FromStr,
	sync::Arc,
};

use api_models::{utils::Uuid, ApiResponse, ErrorType};
use eve_rs::{
	handlebars::Handlebars,
	Context,
	RenderEngine,
	Request,
	Response,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::Transaction;

use super::{audit_logger::AuditLogData, Error};
use crate::{app::App, models::AccessTokenData, Database};

pub struct EveContext {
	request: Request,
	response: Response,
	body_object: Value,
	render_register: Option<Arc<Handlebars<'static>>>,
	state: App,
	db_connection: Option<Transaction<'static, Database>>,
	access_token_data: Option<AccessTokenData>,
	request_id: Uuid,
	audit_log_data: Option<AuditLogData>,
}

impl EveContext {
	pub fn new(request: Request, state: &App) -> Self {
		let render_register = Some(state.render_register.clone());
		EveContext {
			request,
			response: Response::new(),
			body_object: Value::Null,
			render_register,
			state: state.clone(),
			db_connection: None,
			access_token_data: None,
			request_id: Uuid::new_v4(),
			audit_log_data: None,
		}
	}

	pub const fn get_state(&self) -> &App {
		&self.state
	}

	#[allow(dead_code)]
	pub fn get_state_mut(&mut self) -> &mut App {
		&mut self.state
	}

	pub fn get_database_connection(
		&mut self,
	) -> &mut Transaction<'static, Database> {
		self.db_connection.as_mut().unwrap()
	}

	pub fn take_database_connection(&mut self) -> Transaction<'_, Database> {
		self.db_connection.take().unwrap()
	}

	pub async fn commit_database_transaction(
		&mut self,
	) -> Result<(), sqlx::Error> {
		if let Some(transaction) = self.db_connection.take() {
			transaction.commit().await?;
		}
		self.db_connection = Some(self.state.database.begin().await?);

		Ok(())
	}

	pub fn set_database_connection(
		&mut self,
		connection: Transaction<'static, Database>,
	) {
		self.db_connection = Some(connection);
	}

	pub fn get_redis_connection(&mut self) -> &mut RedisConnection {
		&mut self.state.redis
	}

	pub const fn get_body_object(&self) -> &Value {
		&self.body_object
	}

	pub fn get_body_as<TBody>(&self) -> Result<TBody, Error>
	where
		TBody: DeserializeOwned,
	{
		serde_json::from_value(self.body_object.clone())
			.map_err(|err| Error::new(Box::new(err)))
	}

	pub fn success<TBody>(&mut self, data: TBody) -> &mut Self
	where
		TBody: Serialize + Debug,
	{
		self.json(ApiResponse::success(data))
	}

	pub fn error(&mut self, error: ErrorType) -> &mut Self {
		self.json(ApiResponse::error(error))
	}

	#[allow(dead_code)]
	pub fn error_with_message(
		&mut self,
		error: ErrorType,
		message: impl Into<String>,
	) -> &mut Self {
		self.json(ApiResponse::error_with_message(error, message))
	}

	pub fn set_body_object(&mut self, body: Value) {
		self.body_object = body;
	}

	/*
	 * REFACTOR: BREAKING CHANGE
	 * Split get_token_data(&mut self) into two
	 * 		1. get_token_data(&self)
	 *		2. get_mut_token_data(&mut self)
	 */
	pub fn get_token_data(&mut self) -> Option<&mut AccessTokenData> {
		self.access_token_data.as_mut()
	}

	pub fn set_token_data(&mut self, token_data: AccessTokenData) {
		self.access_token_data = Some(token_data);
	}

	pub fn get_param(&self, param_id: &str) -> Option<&String> {
		self.get_request().get_params().get(param_id)
	}

	pub fn get_query_as<TBody>(&self) -> Result<TBody, Error>
	where
		TBody: DeserializeOwned,
	{
		serde_qs::from_str(&self.get_query_string())
			.map_err(|err| Error::new(Box::new(err)))
	}

	/// Get a reference to the unique request id created for current request.
	pub fn get_request_id(&self) -> &Uuid {
		&self.request_id
	}

	/// Get a reference to the audit log data if already set
	#[allow(dead_code)]
	pub fn get_audit_log_data(&self) -> Option<&AuditLogData> {
		self.audit_log_data.as_ref()
	}

	/// Set the audit log data and
	/// returns previously set audit log data if present
	///
	/// ### Note:
	/// Set audit log data to EveContext just before returning success response
	/// from endpoint handler so that audit logs will be added only if the whole
	/// operation is success
	pub fn set_audit_log_data(
		&mut self,
		metadata: AuditLogData,
	) -> Option<AuditLogData> {
		self.audit_log_data.replace(metadata)
	}

	/// Takes the AuditLogData out of the EveContext, leaving a None in its
	/// place.
	pub fn take_audit_log_data(&mut self) -> Option<AuditLogData> {
		self.audit_log_data.take()
	}

	/// Get a mutable reference to the audit log data.
	#[allow(dead_code)]
	pub fn get_mut_audit_log_data(&mut self) -> &mut Option<AuditLogData> {
		&mut self.audit_log_data
	}

	/// Get the request IP address
	pub fn get_request_ip_address(&self) -> IpAddr {
		self.get_header("CF-Connecting-IP")
			.or_else(|| self.get_header("X-Real-IP"))
			.or_else(|| {
				self.get_header("X-Forwarded-For").and_then(|value| {
					value.split(',').next().map(|ip| ip.trim().to_string())
				})
			})
			.and_then(|ip_str| IpAddr::from_str(&ip_str).ok())
			.unwrap_or_else(|| self.get_ip())
	}
}

impl RenderEngine for EveContext {
	fn get_register(&self) -> &Arc<Handlebars> {
		self.render_register.as_ref().unwrap()
	}

	fn set_register(&mut self, register: Arc<Handlebars<'static>>) {
		self.render_register = Some(register);
	}
}

#[cfg(debug_assertions)]
impl Debug for EveContext {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Context: {} - {}", self.get_method(), self.get_path())
	}
}

#[cfg(not(debug_assertions))]
impl Debug for EveContext {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:#?}", self)
	}
}

impl Context for EveContext {
	fn get_request(&self) -> &Request {
		&self.request
	}

	fn get_request_mut(&mut self) -> &mut Request {
		&mut self.request
	}

	fn get_response(&self) -> &Response {
		&self.response
	}

	fn take_response(self) -> Response {
		self.response
	}

	fn get_response_mut(&mut self) -> &mut Response {
		&mut self.response
	}
}
