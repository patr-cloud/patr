use std::borrow::Borrow;

use crate::{
	app::{create_eve_app, App},
	db::{self, generate_new_resource_id},
	error,
	models::{
		error,
		rbac::{self, permissions},
		OneTimeToken,
	},
	pin_fn,
	utils::{
		constants::{self, request_keys},
		get_current_time, EveContext, EveMiddleware,
	},
};
use argon2::Variant;
use db::{
	oauth_insert_into_table_oauth_client,
	oauth_insert_into_table_oauth_client_url,
};
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use uuid::Uuid;

use openssl::base64::encode_block;
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::symm::Cipher;
use pem::{encode, parse, Pem};
use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RSAPrivateKey, RSAPublicKey};
use serde_json::{json, Value};
use surf::url::Url;
use trust_dns_client::client;

// todo
// 1) delete an application, and revoke access of the client
pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/register",
		&[EveMiddleware::CustomFunction(pin_fn!(register))],
	);

	sub_app.post("/auth", &[EveMiddleware::CustomFunction(pin_fn!(auth))]);

	// get list of scope for the given client
	sub_app.get(
		"/:client-id/scope",
		&[EveMiddleware::CustomFunction(pin_fn!(get_scope_list))],
	);

	sub_app.get(
		"/:user-id/user-info",
		&[EveMiddleware::CustomFunction(pin_fn!(get_user_info))],
	);

	sub_app.delete(
		"/:clientId",
		&[EveMiddleware::CustomFunction(pin_fn!(delete_client))],
	);

	// ================ testing middleware
	sub_app.get(
		"/openssl",
		&[EveMiddleware::CustomFunction(pin_fn!(openssl))],
	);

	sub_app
}

// handles user login and consent

// expected format for the request url
// https://api.vicara.com/oauth2?
// response_type=code ==> Right now onely code is supported
// &client_id=29352735982374239857
// &redirect_uri=https://example-app.com/callback
// &scope=create+delete
// &state=xcoivjuywkdkhvusuye3kch
async fn auth(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// usually, scope of client access is taken when the client registers.
	// so here, we can get list of scope.

	// first check if the user has logged in.
	// ask user for consent
	// share the status(bool) and one time use code (Exchange Code) to the client.
	// also add api url, where the client can make calls to get information.
	let query_map = context.get_request().get_query().clone();
	log::debug!("received querry {:?}", &query_map);

	let response_type = query_map.get(request_keys::RESPONSE_TYPE);
	if response_type.is_none() {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let response_type = response_type.unwrap();

	let id = query_map.get(request_keys::ID);
	if id.is_none() {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let id = id.unwrap();
	let id = id.as_bytes();
	//NOTE: If the client_id is invalid, the server should reject the request immediately and display the error to the user.

	let redirect_url = query_map.get(request_keys::REDIRECT_URL);
	if redirect_url.is_none() {
		// first check if only one url is registered in the database. if yes, then use that value, else, return an error.
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let redirect_url = redirect_url.unwrap();

	let scope = query_map.get(request_keys::SCOPE);
	if scope.is_none() {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let scope = scope.unwrap();
	let mut scope: Vec<&str> = scope.split("+").collect();
	// let scope: Vec<&str> = scope.collect();

	let state = query_map.get(request_keys::STATE);
	if state.is_none() {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let state = state.unwrap();

	// TODO: check if the given redirect url exists in the db

	//TODO: ask user for consent

	// generate one time access token
	let unique_id = Uuid::new_v4();
	let unique_id = unique_id.as_bytes();
	let iat = get_current_time();
	let exp = iat + (1000 * 600); // 10 minutes

	let one_time_token = OneTimeToken::new(
		iat,
		exp,
		unique_id.to_vec(),
		id.to_vec(),
		redirect_url.to_string(),
		state.to_string(),
	);
	let jwt = one_time_token
		.to_string(context.get_state().config.jwt_secret.as_str())?;
	let refresh_token = Uuid::new_v4();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));

	// log::debug!(
	// 	"received query is {}, {}, {}, {:#?}, {}",
	// 	&response_type,
	// 	id,
	// 	redirect_url,
	// 	scope,
	// 	state
	// );

	Ok(context)
}

// middleware to get list of scope for the given client id
async fn get_scope_list(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}

// middleware to get user info.
// Note: since this is open id, user info is an additional property.
async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}

// This middleware could also take care of things like
// 1) storing the scope of access the client gets on the resource
// 2) name of the app (done)
// 3) description for the app (optional)
// 4) link to client homepage url
// 5) list of redirect url (done)
async fn register(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();

	// callback url
	let redirect_url = if let Some(Value::Array(redirect_url)) =
		body.get(request_keys::REDIRECT_URL)
	{
		redirect_url
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let name = if let Some(Value::String(name)) = body.get(request_keys::NAME) {
		name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	// generate client id
	let client_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let client_id = client_id.as_bytes();

	// generate secret key for the client
	let mut secret_key = [0; 256];
	rand_bytes(&mut secret_key).unwrap();
	let secret_key = encode_block(&secret_key);

	// hash secret key before storing
	let secret_key_hash = argon2::hash_raw(
		secret_key.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	//once client is registered, add details to database
	db::oauth_insert_into_table_oauth_client(
		context.get_mysql_connection(),
		client_id,
		name,
		&secret_key_hash,
	)
	.await?;

	// add url to table `oatuh_client_url`
	for url in redirect_url.iter() {
		let url = url.as_str().unwrap();
		db::oauth_insert_into_table_oauth_client_url(
			context.get_mysql_connection(),
			url.to_string(),
			client_id,
		)
		.await?;
	}

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ID: client_id,
		request_keys::SECRET_KEY: secret_key,
	}));
	Ok(context)
}

// delete client.
// revoke all access for the users signed in maybe?
async fn delete_client(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}

// testing middleware
async fn openssl(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let rsa = Rsa::generate(1024).unwrap();
	let passphrase = "rust_by_example";
	let private_key: Vec<u8> = rsa
		.private_key_to_pem_passphrase(
			Cipher::aes_128_cbc(),
			passphrase.as_bytes(),
		)
		.unwrap();
	let public_key: Vec<u8> = rsa.public_key_to_pem().unwrap();

	log::debug!("Private key: {}", String::from_utf8(private_key).unwrap());
	log::debug!("Public key: {}", String::from_utf8(public_key).unwrap());

	Ok(context)
}

// HELPER FUNCTIONS

// function to check if given url is a valid url
