use cloudflare::{
	endpoints::{
		workers::{self, CreateRouteParams},
		workerskv,
		zone::custom_hostname::{
			CreateCustomHostname,
			CreateCustomHostnameParams,
			SslParams,
			SslSettingsParams,
		},
	},
	framework::{
		async_api::Client as CloudflareClient,
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};

use crate::utils::{settings::Settings, Error};

fn get_cloudflare_client(config: &Settings) -> Result<CloudflareClient, Error> {
	CloudflareClient::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_token.clone(),
		},
		HttpApiClientConfig::default(),
		Environment::Production,
	)
	.map_err(|err| {
		log::error!("Error while initializing cloudflare client: {}", err);
		Error::empty()
	})
}

pub async fn create_cf_worker_routes_for_domain(
	domain: &str,
	config: &Settings,
) -> Result<String, Error> {
	let cf_client = get_cloudflare_client(config)?;

	let response = cf_client
		.request(&workers::CreateRoute {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			params: CreateRouteParams {
				pattern: format!("*{}/*", domain),
				script: Some(config.cloudflare.worker_script.to_owned()),
			},
		})
		.await?;

	Ok(response.result.id)
}

pub async fn create_cf_custom_hostname(
	host: &str,
	config: &Settings,
) -> Result<String, Error> {
	let cf_client = get_cloudflare_client(config)?;

	let response = cf_client
		.request(&CreateCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			params: CreateCustomHostnameParams {
				hostname: host.to_owned(),
				ssl: SslParams {
					method: "http".to_owned(),
					type_: "dv".to_owned(),
					settings: SslSettingsParams {
						min_tls_version: "1.0".to_owned(),
						..Default::default()
					},
					..Default::default()
				},
			},
		})
		.await?;

	Ok(response.result.id)
}

pub async fn update_kv_for_deployment(
	kv: Vec<workerskv::write_bulk::KeyValuePair>,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config)?;

	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
			bulk_key_value_pairs: kv,
		})
		.await?;

	Ok(())
}

pub async fn update_kv_for_static_site(
	kv: Vec<workerskv::write_bulk::KeyValuePair>,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config)?;

	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
			bulk_key_value_pairs: kv,
		})
		.await?;

	Ok(())
}

pub async fn update_kv_for_managed_url(
	kv: Vec<workerskv::write_bulk::KeyValuePair>,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config)?;

	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
			bulk_key_value_pairs: kv,
		})
		.await?;

	Ok(())
}
