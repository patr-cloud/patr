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
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							ManagedUrlSearchParams {
								sub_domain: sub_domain_filter,
								domain_id: domain_id_filter,
								path: path_filter,
								// TODO nested search params
								url_type: url_type_filter,
								is_active: is_active_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListManagedURLRequest>,
) -> Result<AppResponse<ListManagedURLRequest>, ErrorType> {
	info!("Listing ManagedURLs in workspace `{}`", workspace_id);

	let mut total_count = 0;

	let urls = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.sub_domain,
			managed_url.domain_id,
			managed_url.path,
			managed_url.url_type AS "url_type: ManagedUrlTypeDiscriminant",
			managed_url.deployment_id,
			managed_url.port,
			managed_url.static_site_id,
			managed_url.url,
			managed_url_custom_hostname.is_active,
			managed_url.permanent_redirect,
			managed_url.http_only,
			COUNT(*) OVER() AS "total_count!"
		FROM
			managed_url
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID($2, $3) AS resource
		ON
			managed_url.id = resource.id
		INNER JOIN
			managed_url_custom_hostname
		ON
			managed_url.sub_domain = managed_url_custom_hostname.sub_domain AND
			managed_url.domain_id = managed_url_custom_hostname.domain_id
		WHERE
			managed_url.workspace_id = $1 AND
			managed_url.deleted IS NULL AND
			($4::TEXT IS NULL OR managed_url.sub_domain ILIKE '%' || $4 || '%') AND
			($5::UUID IS NULL OR managed_url.domain_id = $5) AND
			($6::TEXT IS NULL OR managed_url.path ILIKE '%' || $6 || '%') AND
			($7::MANAGED_URL_TYPE IS NULL OR managed_url.url_type = $7) AND
			($8::BOOLEAN IS NULL OR managed_url_custom_hostname.is_active = $8)
		ORDER BY
			resource.created DESC
		LIMIT $9
		OFFSET $10;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::ManagedURL(ManagedURLPermission::View) as _,
		sub_domain_filter as _,
		domain_id_filter as _,
		path_filter as _,
		url_type_filter as _,
		is_active_filter as _,
		count as i64,
		(count * page) as i64,
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
				is_active: row.is_active,
			},
		))
	})
	.collect::<Result<_, ErrorType>>()?;

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListManagedURLResponse { urls })
		.headers(ListManagedURLResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
