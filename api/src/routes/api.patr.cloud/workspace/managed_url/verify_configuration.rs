use axum::http::StatusCode;
use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// Verify if a managed URL's FQDN custom hostname is active on Cloudflare.
///
/// Checks the Cloudflare Custom Hostname status and updates the
/// `managed_url_custom_hostname.is_active` flag accordingly.
pub async fn verify_configuration(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					VerifyManagedURLConfigurationPath {
						workspace_id: _,
						managed_url_id,
					},
				query: (),
				headers:
					VerifyManagedURLConfigurationRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: VerifyManagedURLConfigurationRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, VerifyManagedURLConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedURLConfigurationRequest>, ErrorType> {
	info!("Verifying configuration of ManagedURL");

	let row = query!(
		r#"
		SELECT
			managed_url.sub_domain,
			managed_url.domain_id AS "domain_id: Uuid",
			managed_url_custom_hostname.cloudflare_custom_hostname_id
		FROM
			managed_url
		INNER JOIN
			managed_url_custom_hostname
		ON
			managed_url.sub_domain = managed_url_custom_hostname.sub_domain AND
			managed_url.domain_id = managed_url_custom_hostname.domain_id
		WHERE
			managed_url.id = $1 AND
			managed_url.deleted IS NULL;
		"#,
		managed_url_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let cf_client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?;

	let configured = cf_client
		.request(&EditCustomHostname {
			zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
			custom_hostname_id: &row.cloudflare_custom_hostname_id,
			params: EditCustomHostnameParams {
				custom_metadata: None,
				custom_origin_server: None,
				custom_origin_sni: None,
				ssl: None,
			},
		})
		.await?
		.result
		.status ==
		"active";

	query!(
		r#"
		UPDATE
			managed_url_custom_hostname
		SET
			is_active = $3,
			last_verified = NOW()
		WHERE
			sub_domain = $1 AND
			domain_id = $2;
		"#,
		&row.sub_domain,
		row.domain_id as _,
		configured,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(VerifyManagedURLConfigurationResponse { configured })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
