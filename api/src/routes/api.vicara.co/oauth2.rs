use crate::{
	app::{create_eve_app, App},
	db::{self, generate_new_resource_id},
	error,
	models::{
		error,
		rbac::{self, permissions},
	},
	pin_fn,
	utils::{
		constants::{self, request_keys},
		get_current_time, EveContext, EveMiddleware,
	},
};

use eve_rs::{App as EveApp, Context, Error, NextHandler};

use base64::decode;
use openidconnect::core::{
	CoreClaimName, CoreJwsSigningAlgorithm, CoreProviderMetadata,
	CoreResponseType, CoreSubjectIdentifierType,
};
use openidconnect::{
	AuthUrl, EmptyAdditionalProviderMetadata, IssuerUrl, JsonWebKeySetUrl,
	ResponseTypes, Scope, TokenUrl, UserInfoUrl,
};
use openssl::rsa::{Padding, Rsa};
use openssl::symm::Cipher;
use pem::{encode, parse, Pem};
use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RSAPrivateKey, RSAPublicKey};
use serde_json::{json, Value};
use surf::url::Url;

// sign up
// sign-in
// register client, and give secret key
//

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

	// ================ testing middleware
	sub_app.get(
		"/openssl",
		&[EveMiddleware::CustomFunction(pin_fn!(openssl))],
	);

	sub_app
}

// handles user login and consent
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
// 2) name of the app
async fn register(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// get call back url
	// get key for encryption
	// generate client id
	let body = context.get_body_object().clone();

	// callback url
	let redirect_url = if let Some(Value::String(redirect_url)) =
		body.get(request_keys::REDIRECT_URL)
	{
		redirect_url
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	// name of the client.
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

	let mut rng = OsRng;
	let bits = 2048;
	let private_key = RSAPrivateKey::new(&mut rng, bits);
	if let Err(err) = private_key {
		log::error!("Failed to generate a key. Error {:#?}", err);
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let private_key = private_key.unwrap();

	// try taking out private key components and encode it to base64
	// exponent
	let exponent = private_key.d();

	// prime factors of N
	let primes = private_key.primes();

	// log::debug!("extracted exponent is {:#?}", exponent);
	// log::debug!("extracted primes is {:#?}", primes);
	log::debug!("private key {:#?}", private_key);
	// encode the private key

	// log::debug!("Generated private key is {:#?}", private_key);

	//once client is registered, add details to database
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ID: client_id,
		"private key": "yolo",
	}));
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
