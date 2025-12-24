use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use http::StatusCode;
use models::api::workspace::domain::*;

use crate::prelude::*;

/// The handler to verify a domain in a workspace. This will check if the domain
/// has been verified by checking the DNS records for the required verification
/// record. If the record is found, the domain will be marked as verified.
pub async fn verify_domain_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: VerifyDomainInWorkspacePath {
					workspace_id: _,
					domain_id,
				},
				query: (),
				headers:
					VerifyDomainInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: VerifyDomainInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, VerifyDomainInWorkspaceRequest>,
) -> Result<AppResponse<VerifyDomainInWorkspaceRequest>, ErrorType> {
	info!("Starting: Check to verify domain in workspace");

	let row = query!(
		r#"
		SELECT
			cloudflare_custom_hostname_id
		FROM
			workspace_domain
		WHERE
			id = $1 AND
			workspace_domain.deleted IS NULL;
		"#,
		domain_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let verified = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?
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
	.status == "active";

	if verified {
		query!(
			r#"
			UPDATE
				workspace_domain
			SET
				is_verified = TRUE,
				last_verified = NOW()
			WHERE
				id = $1;
			"#,
			domain_id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	AppResponse::builder()
		.body(VerifyDomainInWorkspaceResponse { verified })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
