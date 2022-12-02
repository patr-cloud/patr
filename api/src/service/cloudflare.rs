use std::collections::HashMap;

use api_models::utils::Uuid;
use cloudflare::{
	endpoints::workerskv,
	framework::{
		async_api::Client as CloudflareClient,
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;

use crate::{
	db,
	models::cloudflare::{CfKey, CfValue, ManagedUrlType},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn get_cloudflare_client(
	config: &Settings,
) -> Result<CloudflareClient, Error> {
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	Ok(client)
}

pub async fn update_cloudflare_kv_routing(
	config: &Settings,
	key: CfKey,
	value: CfValue,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	// as of now, cloudflare kv update api is idempotent
	// if key is present, it will replace existing one
	// if key is absent, it will create new one
	cf_client
		.request_handle(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
			bulk_key_value_pairs: vec![workerskv::write_bulk::KeyValuePair {
				key: key.to_string(),
				value: serde_json::to_string(&value)?,
				expiration: None,
				expiration_ttl: None,
				base64: None,
			}],
		})
		.await?;

	Ok(())
}

pub async fn delete_cloudflare_kv_routing(
	config: &Settings,
	key: CfKey,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	// as of now, cloudflare kv delete api is idempotent
	// if key is present, delete it
	// if key is absent, then it won't throw error
	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}

pub async fn update_cloudflare_kv_routing_for_host(
	connection: &mut <Database as sqlx::Database>::Connection,
	sub_domain: &str,
	domain_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;
	let key = CfKey {
		sub_domain: sub_domain.to_owned(),
		domain: domain.name,
	};

	let all_managed_urls_for_host =
		db::get_all_managed_urls_for_host(connection, sub_domain, domain_id)
			.await?
			.into_iter()
			.filter_map(|managed_url| {
				let key = managed_url.path;
				let value = match managed_url.url_type {
					db::ManagedUrlType::ProxyToDeployment => {
						ManagedUrlType::ProxyDeployment {
							deployment_id: managed_url.deployment_id?,
							port: managed_url.port? as u16,
						}
					}
					db::ManagedUrlType::ProxyToStaticSite => {
						ManagedUrlType::ProxyStaticSite {
							static_site_id: managed_url.static_site_id?,
						}
					}
					db::ManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: managed_url.url?,
					},
					db::ManagedUrlType::Redirect => ManagedUrlType::Redirect {
						url: managed_url.url?,
					},
				};

				Some((key, value))
			})
			.collect::<HashMap<_, _>>();

	// if no managed url is present, the delete the already existing key
	if all_managed_urls_for_host.is_empty() {
		delete_cloudflare_kv_routing(config, key).await?;
	} else {
		update_cloudflare_kv_routing(
			config,
			key,
			CfValue(all_managed_urls_for_host),
		)
		.await?;
	}

	Ok(())
}
