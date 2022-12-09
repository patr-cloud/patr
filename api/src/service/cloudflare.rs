use std::collections::HashMap;

use api_models::utils::Uuid;
use cloudflare::{
	endpoints::{
		workers::{self, CreateRouteParams},
		workerskv,
		zone::custom_hostname::{
			CreateCustomHostname,
			CreateCustomHostnameParams,
			DeleteCustomHostname,
			EditCustomHostname,
			EditCustomHostnameParams,
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
use eve_rs::AsError;

use crate::{
	db,
	models::cloudflare::{
		deployment,
		routing::{self, UrlType},
		static_site,
	},
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

async fn update_kv_for_routing(
	key: routing::Key,
	value: routing::Value,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
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

async fn delete_kv_for_routing(
	key: routing::Key,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}

async fn update_kv_for_deployment(
	key: deployment::Key,
	value: deployment::Value,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request_handle(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
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

#[allow(dead_code)]
async fn delete_kv_for_deployment(
	key: deployment::Key,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}

async fn update_kv_for_static_site(
	key: static_site::Key,
	value: static_site::Value,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request_handle(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
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

#[allow(dead_code)]
async fn delete_kv_for_static_site(
	key: static_site::Key,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}

pub async fn update_cloudflare_kv_for_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	sub_domain: &str,
	domain_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;

	let all_managed_urls_for_host =
		db::get_all_managed_urls_for_host(connection, sub_domain, domain_id)
			.await?
			.into_iter()
			.filter_map(|managed_url| {
				let key = managed_url.path;
				let value = match managed_url.url_type {
					db::ManagedUrlType::ProxyToDeployment => {
						UrlType::ProxyDeployment {
							deployment_id: managed_url.deployment_id?,
						}
					}
					db::ManagedUrlType::ProxyToStaticSite => {
						UrlType::ProxyStaticSite {
							static_site_id: managed_url.static_site_id?,
						}
					}
					db::ManagedUrlType::ProxyUrl => UrlType::ProxyUrl {
						url: managed_url.url?,
					},
					db::ManagedUrlType::Redirect => UrlType::Redirect {
						url: managed_url.url?,
					},
				};

				Some((key, value))
			})
			.collect::<HashMap<_, _>>();

	let key = routing::Key {
		sub_domain: sub_domain.to_owned(),
		domain: domain.name,
	};

	// if no managed url is present, then delete the already existing key
	if all_managed_urls_for_host.is_empty() {
		delete_kv_for_routing(key, config).await?;
	} else {
		let value = routing::Value(all_managed_urls_for_host);
		update_kv_for_routing(key, value, config).await?;
	}

	Ok(())
}

async fn update_cloudflare_kv_for_patr_url(
	resource_id: &Uuid,
	value: routing::UrlType,
	config: &Settings,
) -> Result<(), Error> {
	let key = routing::Key {
		sub_domain: resource_id.to_string(),
		domain: "patr.cloud".to_string(),
	};
	let value = routing::Value(HashMap::from([("/".to_owned(), value)]));

	update_kv_for_routing(key, value, config).await?;

	Ok(())
}

pub async fn update_cloudflare_kv_for_deployment(
	deployment_id: &Uuid,
	value: deployment::Value,
	config: &Settings,
) -> Result<(), Error> {
	let key = deployment::Key(deployment_id.to_owned());
	// todo: Is it okay to update routing every time?
	// todo: for stop/delete page use ttl of 7-15 days from here itself
	update_cloudflare_kv_for_patr_url(
		deployment_id,
		routing::UrlType::ProxyDeployment {
			deployment_id: deployment_id.clone(),
		},
		config,
	)
	.await?;
	update_kv_for_deployment(key, value, config).await?;

	Ok(())
}

pub async fn update_cloudflare_kv_for_static_site(
	static_site_id: &Uuid,
	value: static_site::Value,
	config: &Settings,
) -> Result<(), Error> {
	let key = static_site::Key(static_site_id.to_owned());
	// todo: Is it okay to update routing every time?
	// todo: for stop/delete page use ttl of 7-15 days from here itself
	update_cloudflare_kv_for_patr_url(
		static_site_id,
		routing::UrlType::ProxyStaticSite {
			static_site_id: static_site_id.clone(),
		},
		config,
	)
	.await?;
	update_kv_for_static_site(key, value, config).await?;

	Ok(())
}

pub async fn add_domain_to_cloudflare_worker_routes(
	host: &str,
	config: &Settings,
) -> Result<String, Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request_handle(&workers::CreateRoute {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			params: CreateRouteParams {
				pattern: format!("*{}/*", host),
				script: Some(config.cloudflare.worker_script.to_owned()),
			},
		})
		.await?;

	Ok(response.result.id)
}

pub async fn delete_domain_from_cloudflare_worker_routes(
	route_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	cf_client
		.request_handle(&workers::DeleteRoute {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: route_id,
		})
		.await?;

	Ok(())
}

pub async fn add_custom_hostname_to_cloudflare(
	host: &str,
	config: &Settings,
) -> Result<(String, String), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request_handle(&CreateCustomHostname {
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

	Ok((response.result.id, response.result.status))
}

pub async fn delete_custom_hostname_from_cloudflare(
	custom_hostname_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	cf_client
		.request_handle(&DeleteCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: custom_hostname_id,
		})
		.await?;

	Ok(())
}

pub async fn refresh_custom_hostname_in_cloudflare(
	custom_hostname_id: &str,
	config: &Settings,
) -> Result<String, Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request_handle(&EditCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: custom_hostname_id,
			params: EditCustomHostnameParams {
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

	Ok(response.result.status)
}
