use hickory_resolver::{Resolver, config::ResolverConfig, name_server::TokioConnectionProvider};
use http::StatusCode;
use models::api::workspace::domain::*;

use crate::prelude::*;

/// The handler to verify a domain in a workspace. This checks if the user has
/// added a TXT record at `_patr-verify.{domain}` with the domain ID as the
/// value. If the record is found, the domain will be marked as verified.
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
		state: _,
	}: AuthenticatedAppRequest<'_, VerifyDomainInWorkspaceRequest>,
) -> Result<AppResponse<VerifyDomainInWorkspaceRequest>, ErrorType> {
	info!("Starting: Check to verify domain in workspace");

	let row = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			tld
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

	let verification_hostname = format!("_patr-verify.{}.{}", row.name, row.tld);
	let expected_value = row.id.to_string();

	let resolver = Resolver::builder_with_config(
		ResolverConfig::default(),
		TokioConnectionProvider::default(),
	)
	.build();

	let verified = match resolver.txt_lookup(&verification_hostname).await {
		Ok(lookup) => lookup.iter().any(|txt| {
			txt.iter()
				.any(|data| String::from_utf8_lossy(data) == expected_value)
		}),
		Err(_) => false,
	};

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
