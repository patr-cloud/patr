use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

use eve_rs::{
	handlebars::Handlebars,
	Context,
	RenderEngine,
	Request,
	Response,
};
use serde_json::Value;
use sqlx::Transaction;

use crate::{app::App, models::AccessTokenData, Database};

pub struct EveContext {
	request: Request,
	response: Response,
	body_object: Value,
	render_register: Option<Arc<Handlebars<'static>>>,
	state: App,
	db_connection: Option<Transaction<'static, Database>>,
	access_token_data: Option<AccessTokenData>,
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
		}
	}

	pub const fn get_state(&self) -> &App {
		&self.state
	}

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

	pub fn set_database_connection(
		&mut self,
		connection: Transaction<'static, Database>,
	) {
		self.db_connection = Some(connection);
	}

	pub const fn get_body_object(&self) -> &Value {
		&self.body_object
	}

	pub fn set_body_object(&mut self, body: Value) {
		self.body_object = body;
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
		write!(
			f,
			"Context: {} - {}",
			self.get_method().to_string(),
			self.get_path()
		)
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
