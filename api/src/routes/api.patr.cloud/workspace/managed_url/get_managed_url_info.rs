use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to get the information of a single managed URL in a workspace.
pub async fn get_managed_url_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetManagedURLInfoPath {
					workspace_id,
					managed_url_id,
				},
				query: (),
				headers:
					GetManagedURLInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetManagedURLInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetManagedURLInfoRequest>,
) -> Result<AppResponse<GetManagedURLInfoRequest>, ErrorType> {
	info!("Getting ManagedURL `{}`", managed_url_id);

	let row = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.sub_domain,
			managed_url.domain_id,
			managed_url.path,
			managed_url.url_type AS "url_type: ManagedUrlTypeDiscriminant",
			managed_url.deployment_id,
			managed_url.port,
			managed_url.url,
			managed_url_custom_hostname.is_active,
			managed_url.permanent_redirect,
			managed_url.http_only
		FROM
			managed_url
		INNER JOIN
			managed_url_custom_hostname
		ON
			managed_url.sub_domain = managed_url_custom_hostname.sub_domain AND
			managed_url.domain_id = managed_url_custom_hostname.domain_id
		WHERE
			managed_url.id = $1 AND
			managed_url.workspace_id = $2 AND
			managed_url.deleted IS NULL;
		"#,
		managed_url_id,
		workspace_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let managed_url = WithId::new(
		row.id,
		ManagedUrl {
			sub_domain: row.sub_domain,
			domain_id: row.domain_id,
			path: row.path,
			url_type: match row.url_type {
				ManagedUrlTypeDiscriminant::ProxyUrl => ManagedUrlType::ProxyUrl {
					url: row
						.url
						.ok_or_else(|| ErrorType::server_error("url in db is NULL"))?,
					http_only: row
						.http_only
						.ok_or_else(|| ErrorType::server_error("http_only in db is NULL"))?,
				},
				ManagedUrlTypeDiscriminant::Redirect => ManagedUrlType::Redirect {
					url: row
						.url
						.ok_or_else(|| ErrorType::server_error("url in db is NULL"))?,
					permanent_redirect: row.permanent_redirect.ok_or_else(|| {
						ErrorType::server_error("permanent_redirect in db is NULL")
					})?,
					http_only: row
						.http_only
						.ok_or_else(|| ErrorType::server_error("http_only in db is NULL"))?,
				},
				ManagedUrlTypeDiscriminant::ProxyDeployment => ManagedUrlType::ProxyDeployment {
					deployment_id: row
						.deployment_id
						.ok_or_else(|| ErrorType::server_error("deployment_id in db is NULL"))?,
					port: row
						.port
						.ok_or_else(|| ErrorType::server_error("port in db is NULL"))?
						as u16,
				},
			},
			is_active: row.is_active,
		},
	);

	AppResponse::builder()
		.body(GetManagedURLInfoResponse { managed_url })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
