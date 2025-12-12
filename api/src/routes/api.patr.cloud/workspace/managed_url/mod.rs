use std::collections::BTreeMap;

use axum::Router;
use cloudflare::{
	endpoints::workerskv::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::api::workspace::managed_url::ManagedUrlTypeDiscriminant;

use crate::{prelude::*, utils::config::AppConfig};

mod create_managed_url;
mod delete_managed_url;
mod list_managed_url;
mod update_managed_url;
#[allow(unreachable_code, unused_variables)]
mod verify_configuration;

use self::{
	create_managed_url::*,
	delete_managed_url::*,
	list_managed_url::*,
	update_managed_url::*,
	verify_configuration::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_managed_url, state, allowed_client_type)
		.mount_auth_endpoint(delete_managed_url, state, allowed_client_type)
		.mount_auth_endpoint(list_managed_url, state, allowed_client_type)
		.mount_auth_endpoint(update_managed_url, state, allowed_client_type)
		.mount_auth_endpoint(verify_configuration, state, allowed_client_type)
}

async fn sync_worker_kv_for_domain(
	domain: &str,
	database: &mut DatabaseConnection,
	config: &AppConfig,
) -> Result<(), ErrorType> {
	use models::cloudflare::kv::ManagedUrlKVData::*;

	let client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?;

	let kv_body = query!(
		r#"
		SELECT
			managed_url.path,
			managed_url.url_type AS "url_type: ManagedUrlTypeDiscriminant",
			managed_url.deployment_id AS "deployment_id: Uuid",
			deployment.runner AS "runner?: Uuid",
			managed_url.port,
			managed_url.static_site_id AS "static_site_id: Uuid",
			managed_url.url,
			managed_url.is_configured,
			managed_url.permanent_redirect,
			managed_url.http_only
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		LEFT JOIN
			deployment
		ON
			managed_url.deployment_id = deployment.id
		WHERE
			managed_url.deleted IS NULL AND
			CONCAT(
				managed_url.sub_domain,
				'.',
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) = $1;
		"#,
		domain as _,
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| {
		Ok((
			row.path,
			match row.url_type {
				ManagedUrlTypeDiscriminant::ProxyDeployment => ProxyDeployment {
					deployment_id: row.deployment_id.ok_or(ErrorType::server_error(
						"deployment_id is NULL when it's a proxy deployment",
					))?,
					port: row
						.port
						.map(|port| port as u16)
						.ok_or(ErrorType::server_error(
							"port is NULL when it's a proxy deployment",
						))?,
					runner_id: row
						.runner
						.ok_or(ErrorType::server_error("Cannot find runner_id"))?,
				},
				ManagedUrlTypeDiscriminant::ProxyStaticSite => ProxyStaticSite {
					static_site_id: row.static_site_id.ok_or(ErrorType::server_error(
						"static_site_id is NULL when it's a proxy static site",
					))?,
					upload_id: Uuid::nil(), /* TODO Placeholder, replace with actual upload_id
					                         * retrieval when needed */
				},
				ManagedUrlTypeDiscriminant::ProxyUrl => ProxyUrl {
					url: row
						.url
						.ok_or(ErrorType::server_error("url is NULL when it's a proxy url"))?,
					http_only: row.http_only.ok_or(ErrorType::server_error(
						"http_only is NULL when it's a proxy url",
					))?,
				},
				ManagedUrlTypeDiscriminant::Redirect => Redirect {
					url: row
						.url
						.ok_or(ErrorType::server_error("url is NULL when it's a redirect"))?,
					permanent_redirect: row.permanent_redirect.ok_or(ErrorType::server_error(
						"permanent_redirect is NULL when it's a redirect",
					))?,
					http_only: row.http_only.ok_or(ErrorType::server_error(
						"http_only is NULL when it's a redirect",
					))?,
				},
			},
		))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	if kv_body.is_empty() {
		client
			.request(&delete_key::DeleteKey {
				account_identifier: &config.cloudflare.account_id,
				namespace_identifier: &config.cloudflare.worker_namespace_id,
				key: &domain,
			})
			.await?;
	} else {
		client
			.request(&write_key::WriteKey {
				account_identifier: &config.cloudflare.account_id,
				namespace_identifier: &config.cloudflare.worker_namespace_id,
				key: &domain,
				params: write_key::WriteKeyParams {
					expiration: None,
					expiration_ttl: None,
				},
				body: write_key::WriteKeyBody::Value(serde_json::to_vec(&kv_body)?),
			})
			.await?;
	}

	Ok(())
}
