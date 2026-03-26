use std::time::Duration;

use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// Verify if a managed URL is actively being served by Patr.
///
/// Makes an HTTP request to `https://{fqdn}/.well-known/patr/managed-url`
/// and checks if the response is 200 OK (meaning the ingress worker is
/// handling this host).
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
		state: _,
	}: AuthenticatedAppRequest<'_, VerifyManagedURLConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedURLConfigurationRequest>, ErrorType> {
	info!("Verifying configuration of ManagedURL");

	let row = query!(
		r#"
		SELECT
			managed_url.sub_domain,
			workspace_domain.name AS domain_name,
			workspace_domain.tld AS domain_tld
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		WHERE
			managed_url.id = $1 AND
			managed_url.deleted IS NULL;
		"#,
		managed_url_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let fqdn = if row.sub_domain == "@" {
		format!("{}.{}", row.domain_name, row.domain_tld)
	} else {
		format!("{}.{}.{}", row.sub_domain, row.domain_name, row.domain_tld)
	};

	let configured = reqwest::Client::new()
		.get(format!("https://{}/.well-known/patr/managed-url", fqdn))
		.header("Cache-Control", "no-cache")
		.timeout(Duration::from_secs(10))
		.send()
		.await
		.map(|resp| resp.status().is_success())
		.unwrap_or(false);

	query!(
		r#"
		UPDATE
			managed_url
		SET
			is_active = $2
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
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
