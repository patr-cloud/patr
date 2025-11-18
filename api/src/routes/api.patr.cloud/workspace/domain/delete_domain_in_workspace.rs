use cloudflare::{
	endpoints::zones::zone::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::api::workspace::domain::*;
use reqwest::StatusCode;

use crate::prelude::*;

#[instrument(skip(database, config))]
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
		config,
		user_data: _,
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
		CloudflareClient::new(
			Credentials::UserAuthToken {
				token: config.cloudflare.api_key.clone(),
			},
			ClientConfig::default(),
			Environment::Production,
		)?
		.request(&DeleteZone { identifier: &zone })
		.await?;
	}

	AppResponse::builder()
		.body(DeleteDomainInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
