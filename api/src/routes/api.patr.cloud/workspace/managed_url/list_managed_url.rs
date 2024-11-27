use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to list all managed URLs in a workspace. This will return all
/// managed URLs that the user has access to in the workspace.
pub async fn list_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListManagedURLPath { workspace_id },
				query:
					Paginated {
						data:
							ListManagedURLQuery {
								order, // TODO implement these
								order_by,
								filter,
							},
						count,
						page,
					},
				headers:
					ListManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListManagedURLRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListManagedURLRequest>,
) -> Result<AppResponse<ListManagedURLRequest>, ErrorType> {
	info!("Listing ManagedURLs in workspace `{}`", workspace_id);

	let mut total_count = 0;

	let urls = query!(
		r#"
		SELECT
			managed_url.id,
			sub_domain,
			domain_id,
			path,
			url_type as "url_type: ManagedUrlTypeDiscriminant",
			deployment_id,
			port,
			static_site_id,
			url,
			is_configured,
			permanent_redirect,
			http_only,
			COUNT(*) OVER() AS "total_count!"
		FROM
			managed_url
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			managed_url.id = resource.id
		WHERE
			workspace_id = $1 AND
			managed_url.deleted IS NULL
		ORDER BY
			resource.created DESC
		LIMIT $4
		OFFSET $5;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::ManagedURL(ManagedURLPermission::View) as _,
		count as i32,
		(count * page) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		Ok(WithId::new(
			row.id,
			ManagedUrl {
				sub_domain: row.sub_domain,
				domain_id: row.domain_id.into(),
				path: row.path,
				url_type: match row.url_type {
					ManagedUrlTypeDiscriminant::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: row
							.url
							.ok_or(ErrorType::server_error("url in db is NULL"))?,
						http_only: row
							.http_only
							.ok_or(ErrorType::server_error("http_only in db is NULL"))?,
					},
					ManagedUrlTypeDiscriminant::Redirect => ManagedUrlType::Redirect {
						url: row
							.url
							.ok_or(ErrorType::server_error("url in db is NULL"))?,
						permanent_redirect: row
							.permanent_redirect
							.ok_or(ErrorType::server_error("permanent_redirect in db is NULL"))?,
						http_only: row
							.http_only
							.ok_or(ErrorType::server_error("http_only in db is NULL"))?,
					},
					ManagedUrlTypeDiscriminant::ProxyStaticSite => {
						ManagedUrlType::ProxyStaticSite {
							static_site_id: row
								.static_site_id
								.ok_or(ErrorType::server_error("static_site_id in db is NULL"))?
								.into(),
						}
					}
					ManagedUrlTypeDiscriminant::ProxyDeployment => {
						ManagedUrlType::ProxyDeployment {
							deployment_id: row
								.deployment_id
								.ok_or(ErrorType::server_error("deployment_id in db is NULL"))?
								.into(),
							port: row
								.port
								.ok_or(ErrorType::server_error("port in db is NULL"))?
								as u16,
						}
					}
				},
				is_configured: row.is_configured,
			},
		))
	})
	.collect::<Result<_, ErrorType>>()?;

	AppResponse::builder()
		.body(ListManagedURLResponse { urls })
		.headers(ListManagedURLResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
