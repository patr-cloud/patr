use crate::{app::App, models::AccessTokenData};
use eve_rs::{Context, Request, Response};
use serde_json::Value;
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};
use std::fmt::{Debug, Formatter};

pub struct EveContext {
	request: Request,
	response: Response,
	body_object: Option<Value>,
	state: App,
	db_connection: Option<Transaction<PoolConnection<MySqlConnection>>>,
	access_token_data: Option<AccessTokenData>,
}

impl EveContext {
	pub fn new(request: Request, state: App) -> Self {
		EveContext {
			request,
			response: Response::new(),
			body_object: None,
			state,
			db_connection: None,
			access_token_data: None,
		}
	}

	pub fn get_state(&self) -> &App {
		&self.state
	}

	pub fn get_db_connection(
		&mut self,
	) -> &mut Transaction<PoolConnection<MySqlConnection>> {
		self.db_connection.as_mut().unwrap()
	}

	pub fn take_db_connection(
		&mut self,
	) -> Transaction<PoolConnection<MySqlConnection>> {
		self.db_connection.take().unwrap()
	}

	pub fn set_db_connection(
		&mut self,
		connection: Transaction<PoolConnection<MySqlConnection>>,
	) {
		self.db_connection = Some(connection);
	}

	pub fn get_body_object(&self) -> Option<&Value> {
		self.body_object.as_ref()
	}

	pub fn set_body_object(&mut self, body: Value) {
		self.body_object = Some(body);
	}

	pub fn get_token_data(&mut self) -> Option<&mut AccessTokenData> {
		self.access_token_data.as_mut()
	}

	pub fn set_token_data(&mut self, token_data: AccessTokenData) {
		self.access_token_data = Some(token_data);
	}

	pub fn get_param(&self, param_id: &str) -> Option<&String> {
		self.get_request().get_params().get(param_id)
	}
}

impl Debug for EveContext {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Context: {} - {}",
			self.get_method().to_string(),
			self.get_path()
		)
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

impl Clone for EveContext {
	fn clone(&self) -> Self {
		EveContext {
			request: self.request.clone(),
			response: self.response.clone(),
			body_object: self.body_object.clone(),
			state: self.state.clone(),
			db_connection: None,
			access_token_data: self.access_token_data.clone(),
		}
	}
}
