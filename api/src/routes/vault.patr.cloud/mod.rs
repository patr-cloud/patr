use std::net::{IpAddr, Ipv4Addr};

use api_models::utils::Uuid;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use vaultrs::{
	client::{VaultClient, VaultClientSettingsBuilder},
	kv1,
};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		rbac::permissions,
		secret::{
			VaultResponse,
			VaultResponseData,
			VaultResponseMetadata,
			VaultSecretResponse,
		},
		UserAuthenticationData,
	},
	pin_fn,
	routes::get_request_ip_address,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/v1/secret/data/:workspaceId/:secretId",
		[EveMiddleware::CustomFunction(pin_fn!(middle_man_fn))],
	);

	sub_app
}

async fn middle_man_fn(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Checking if token has access to resource",
		request_id,
	);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();

	let resource =
		db::get_resource_by_id(context.get_database_connection(), &secret_id)
			.await?
			.filter(|value| value.owner_id == workspace_id)
			.status(401)?;

	let token = context
		.get_header("x-vault-token")
		.status(401)
		.body(error!(UNAUTHORIZED).to_string())?;

	let ip_addr = get_request_ip_address(&context)
		.parse()
		.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

	let jwt_secret = context.get_state().config.jwt_secret.clone();
	let mut redis_conn = context.get_redis_connection().clone();
	let token_data = UserAuthenticationData::parse(
		context.get_database_connection(),
		&mut redis_conn,
		&jwt_secret,
		&token,
		&ip_addr,
	)
	.await?;

	if !token_data.has_access_for_requested_action(
		&resource.owner_id,
		&resource.id,
		permissions::workspace::secret::INFO,
	) {
		return Err(Error::empty()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string()));
	}

	log::trace!(
		"request_id: {} - Calling Vault API to get secrets",
		request_id,
	);

	let config = context.get_state().config.clone();
	let vault_client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address(&config.vault.upstream_host)
			.token(&config.vault.token)
			.build()?,
	)?;

	let vault_response = kv1::get_raw(
		&vault_client,
		"secret",
		&format!("data/{}/{}", workspace_id.as_str(), secret_id.as_str()),
	)
	.await?;

	let vault_secret =
		serde_json::from_value::<VaultResponse>(vault_response.data)?;

	context.json(VaultSecretResponse {
		data: serde_json::to_value(VaultResponse {
			data: VaultResponseData {
				data: vault_secret.data.data,
			},
			metadata: {
				VaultResponseMetadata {
					created_time: vault_secret.metadata.created_time,
					custom_metadata: None,
					deletion_time: String::from(""),
					destroyed: false,
					version: vault_secret.metadata.version,
				}
			},
		})?,
		auth: None,
		lease_duration: 0,
		lease_id: String::from(""),
		renewable: false,
		request_id: vault_response.request_id,
	});
	Ok(context)
}
