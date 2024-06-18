use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to update a managed URL. At the moment, only the URL can be
/// updated. However, this will be expanded in the future. At least one
/// parameter must be provided for the update.
pub async fn update_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateManagedURLPath {
					workspace_id,
					managed_url_id,
				},
				query: (),
				headers:
					UpdateManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					UpdateManagedURLRequestProcessed {
						path,
						url_type: managed_url_type,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, UpdateManagedURLRequest>,
) -> Result<AppResponse<UpdateManagedURLRequest>, ErrorType> {
	info!("Creating ManagedURL with ID: `{}`", managed_url_id);

	// Check to make sure that the Managed URL exist
	query!(
		r#"
        SELECT
            managed_url.id
        FROM
            managed_url
        INNER JOIN
            resource
        ON
            managed_url.id = resource.id
        WHERE
            managed_url.id = $1 AND
            managed_url.deleted IS NULL AND
            resource.owner_id = $2;
        "#,
		managed_url_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let path = format!("/{}", path.trim_start_matches('/'));

	let url_type;
	let deployment_id;
	let port;
	let static_site_id;
	let url;
	let permanent_redirect;
	let http_only;

	match managed_url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id: managed_url_deployment_id,
			port: managed_url_port,
		} => {
			url_type = "proxy_deployment";
			deployment_id = Some(managed_url_deployment_id);
			port = Some(managed_url_port);
			static_site_id = None;
			url = None;
			permanent_redirect = None;
			http_only = None;
		}
		ManagedUrlType::ProxyStaticSite {
			static_site_id: managed_url_static_site_id,
		} => {
			url_type = "proxy_static_site";
			deployment_id = None;
			port = None;
			static_site_id = Some(managed_url_static_site_id);
			url = None;
			permanent_redirect = None;
			http_only = None;
		}
		ManagedUrlType::ProxyUrl {
			url: managed_url_url,
			http_only: managed_url_http_only,
		} => {
			url_type = "proxy_url";
			deployment_id = None;
			port = None;
			static_site_id = None;
			url = Some(managed_url_url);
			permanent_redirect = None;
			http_only = Some(managed_url_http_only);
		}
		ManagedUrlType::Redirect {
			url: managed_url_url,
			permanent_redirect: managed_url_permanent_redirect,
			http_only: managed_url_http_only,
		} => {
			url_type = "redirect";
			deployment_id = None;
			port = None;
			static_site_id = None;
			url = Some(managed_url_url);
			permanent_redirect = Some(managed_url_permanent_redirect);
			http_only = Some(managed_url_http_only);
		}
	}
	query!(
		r#"
		UPDATE
			managed_url
		SET
			path = $2,
			url_type = $3,
			deployment_id = $4,
			port = $5,
			static_site_id = $6,
			url = $7,
			permanent_redirect = $8,
			http_only = $9
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
		path,
		url_type as _,
		deployment_id as _,
		port.map(|port| port as i32),
		static_site_id as _,
		url,
		permanent_redirect,
		http_only,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateManagedURLResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
