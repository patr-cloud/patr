use crate::app::App;
use bytes::Bytes;
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};
use std::collections::HashMap;
use thruster::{
	middleware::{
		cookies::{Cookie, CookieOptions, HasCookies, SameSite},
		query_params::HasQueryParams,
	},
	Context, Request, Response,
};

pub struct ThrusterContext {
	response: Response,
	cookies: Vec<Cookie>,
	query_params: HashMap<String, String>,
	request: Request,
	status: u32,
	headers: HashMap<String, String>,
	params: HashMap<String, String>,
	db_connection: Option<Transaction<PoolConnection<MySqlConnection>>>,
	state: App,
}

#[allow(dead_code)]
impl ThrusterContext {
	pub fn new(request: Request, state: App) -> Self {
		let mut ctx = ThrusterContext {
			response: Response::new(),
			cookies: Vec::new(),
			query_params: HashMap::new(),
			request,
			headers: HashMap::new(),
			params: HashMap::new(),
			status: 200,
			db_connection: None,
			state,
		};
		ctx.headers = ctx.request.headers();
		ctx.params = ctx.request.params.clone();
		let body = ctx.request.body().to_owned();
		ctx.body(&body);

		ctx.set("Server", "Thruster");

		ctx
	}

	pub fn from_response(response: Response, state: App) -> Self {
		ThrusterContext {
			response,
			cookies: vec![],
			query_params: HashMap::new(),
			request: Request::new(),
			headers: HashMap::new(),
			params: HashMap::new(),
			status: 200,
			db_connection: None,
			state
		}
	}

	pub fn body(&mut self, body_string: &str) {
		self.response
			.body_bytes_from_vec(body_string.as_bytes().to_vec());
	}

	pub fn get_body(&self) -> String {
		std::str::from_utf8(&self.response.response)
			.unwrap_or("")
			.to_owned()
	}

	pub fn get_status(&self) -> u32 {
		self.status
	}

	pub fn status(&mut self, code: u32) {
		self.status = code;
	}

	pub fn content_type(&mut self, c_type: &str) {
		self.set("Content-Type", c_type);
	}

	pub fn redirect(&mut self, destination: &str) {
		self.status(302);

		self.set("Location", destination);
	}

	pub fn cookie(&mut self, name: &str, value: &str, options: &CookieOptions) {
		let cookie_value = match self.headers.get("Set-Cookie") {
			Some(val) => format!("{}, {}", val, self.cookify_options(name, value, &options)),
			None => self.cookify_options(name, value, &options),
		};

		self.set("Set-Cookie", &cookie_value);
	}

	pub fn header(&self, name: &str) -> Option<&str> {
		if let Some(header) = self.headers.get(name) {
			Some(header)
		} else {
			None
		}
	}

	pub fn param(&self, name: &str) -> Option<&String> {
		self.params.get(name)
	}

	pub fn method(&self) -> &str {
		self.request.method()
	}

	pub fn url(&self) -> &str {
		self.request.path()
	}

	pub fn get_state(&self) -> &App {
		&self.state
	}

	pub fn set_db_connection(&mut self, connection: Transaction<PoolConnection<MySqlConnection>>) {
		self.db_connection = Some(connection);
	}

	pub fn get_db_connection(&mut self) -> &mut Transaction<PoolConnection<MySqlConnection>> {
		self.db_connection.as_mut().unwrap()
	}

	pub fn take_db_connection(&mut self) -> Transaction<PoolConnection<MySqlConnection>> {
		self.db_connection.take().unwrap()
	}

	pub fn get_request(&self) -> &Request {
		&self.request
	}

	fn cookify_options(&self, name: &str, value: &str, options: &CookieOptions) -> String {
		let mut pieces = vec![format!("Path={}", options.path)];

		if options.expires > 0 {
			pieces.push(format!("Expires={}", options.expires));
		}

		if options.max_age > 0 {
			pieces.push(format!("Max-Age={}", options.max_age));
		}

		if !options.domain.is_empty() {
			pieces.push(format!("Domain={}", options.domain));
		}

		if options.secure {
			pieces.push("Secure".to_owned());
		}

		if options.http_only {
			pieces.push("HttpOnly".to_owned());
		}

		if let Some(ref same_site) = options.same_site {
			match same_site {
				SameSite::Strict => pieces.push("SameSite=Strict".to_owned()),
				SameSite::Lax => pieces.push("SameSite=Lax".to_owned()),
			};
		}

		format!("{}={}; {}", name, value, pieces.join(", "))
	}
}

impl Context for ThrusterContext {
	type Response = Response;

	fn get_response(mut self) -> Self::Response {
		self.response.status_code(self.status, "");

		self.response
	}

	fn set_body(&mut self, body: Vec<u8>) {
		self.response.body_bytes_from_vec(body);
	}

	fn set_body_bytes(&mut self, body_bytes: Bytes) {
		self.response.body_bytes(&body_bytes);
	}

	fn route(&self) -> &str {
		self.request.path()
	}

	fn set(&mut self, key: &str, value: &str) {
		self.headers.insert(key.to_owned(), value.to_owned());
	}

	fn remove(&mut self, key: &str) {
		self.headers.remove(key);
	}
}

impl HasQueryParams for ThrusterContext {
	fn set_query_params(&mut self, query_params: HashMap<String, String>) {
		self.query_params = query_params;
	}
}

impl HasCookies for ThrusterContext {
	fn set_cookies(&mut self, cookies: Vec<Cookie>) {
		self.cookies = cookies;
	}

	fn headers(&self) -> HashMap<String, String> {
		self.request.headers()
	}
}
