use std::fmt::{Debug, Formatter};

use api_models::{ApiResponse, ErrorType};
use eve_rs::{Context, Request, Response};
use redis::aio::MultiplexedConnection as RedisConnection;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::Transaction;

use super::Error;
use crate::{app::App, models::UserAuthenticationData, Database};

pub struct EveContext {
	request: Request,
	response: Response,
	body_object: Value,
	state: App,
	db_connection: Option<Transaction<'static, Database>>,
	user_auth_data: Option<UserAuthenticationData>,
}

impl EveContext {
	pub fn new(request: Request, state: &App) -> Self {
		EveContext {
			request,
			response: Response::new(),
			body_object: Value::Null,
			state: state.clone(),
			db_connection: None,
			user_auth_data: None,
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

	pub fn get_token_data(&mut self) -> Option<&mut UserAuthenticationData> {
		self.user_auth_data.as_mut()
	}

	pub fn set_token_data(&mut self, token_data: UserAuthenticationData) {
		self.user_auth_data = Some(token_data);
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
}

impl Debug for EveContext {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Context: {} - {}", self.get_method(), self.get_path())
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
