use cloudflare::{
	endpoints::zones::{custom_hostnames::*, zone::*},
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
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

	let row = query!(
		r#"
		DELETE FROM
			workspace_domain
		WHERE
			id = $1
		RETURNING
			cloudflare_custom_hostname_id;
		"#,
		domain_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	let client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?;

	client
		.request(&DeleteCustomHostname {
			zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
			custom_hostname_id: &row.cloudflare_custom_hostname_id,
		})
		.await?;

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
		client.request(&DeleteZone { identifier: &zone }).await?;
	}

	AppResponse::builder()
		.body(DeleteDomainInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
