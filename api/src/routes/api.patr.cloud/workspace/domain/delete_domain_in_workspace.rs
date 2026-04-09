use cloudflare::{
	endpoints::zones::zone::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiFailure,
	},
};
use models::api::workspace::domain::*;
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn delete_domain_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDomainInWorkspacePath {
					workspace_id,
					domain_id,
				},
				query: (),
				headers:
					DeleteDomainInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteDomainInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteDomainInWorkspaceRequest>,
) -> Result<AppResponse<DeleteDomainInWorkspaceRequest>, ErrorType> {
	info!("Deleting domain `{domain_id}` in workspace `{workspace_id}`");

	query!(
		r#"
        DELETE FROM
            user_controlled_domain
        WHERE
            domain_id = $1;
        "#,
		domain_id as _
	)
	.execute(&mut **database)
	.await?;

	let zone = query!(
		r#"
        DELETE FROM
            patr_controlled_domain
        WHERE
            domain_id = $1
        RETURNING zone_identifier;
        "#,
		domain_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|r| r.zone_identifier);

	// This will fail with ResourceInUse if managed URLs (or their custom
	// hostnames) still reference this domain. The user must delete all managed
	// URLs first — doing so automatically cleans up the CF custom hostnames.
	query!(
		r#"
		DELETE FROM
			workspace_domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	// Mark the resource as deleted in the database
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut **database)
	.await?;

	if let Some(zone) = zone {
		let client = CloudflareClient::new(
			Credentials::UserAuthToken {
				token: state.config.cloudflare.api_key.clone(),
			},
			ClientConfig::default(),
			Environment::Custom(state.config.cloudflare.base_url.clone()),
		)?;

		match client.request(&DeleteZone { identifier: &zone }).await {
			Ok(_) => {}
			Err(ApiFailure::Error(status, _)) if status == reqwest::StatusCode::NOT_FOUND => {}
			Err(err) => return Err(ErrorType::server_error(err)),
		}
	}

	AppResponse::builder()
		.body(DeleteDomainInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
