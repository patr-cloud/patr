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
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, UpdateManagedURLRequest>,
) -> Result<AppResponse<UpdateManagedURLRequest>, ErrorType> {
	info!("Updating ManagedURL with ID: `{}`", managed_url_id);

	let managed_url = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.sub_domain,
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain!",
			managed_url.path
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		WHERE
			managed_url.id = $1 AND
			managed_url.deleted IS NULL AND
			managed_url.workspace_id = $2;
		"#,
		managed_url_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let path = path.map(|path| format!("/{}", path.trim_start_matches('/')));

	let url_type;
	let deployment_id;
	let port;
	let static_site_id;
	let url;
	let permanent_redirect;
	let http_only;

	let mut runner_id_to_update = None;

	if let Some(managed_url_type) = managed_url_type {
		match managed_url_type {
			ManagedUrlType::ProxyDeployment {
				deployment_id: managed_url_deployment_id,
				port: managed_url_port,
			} => {
				let deployment = query!(
					r#"
					SELECT
						deployment.runner AS "runner: Uuid"
					FROM
						deployment
					INNER JOIN
						resource
					ON
						deployment.id = resource.id
					WHERE
						deployment.id = $1 AND
						deployment.deleted IS NULL AND
						resource.owner_id = $2;
					"#,
					managed_url_deployment_id as _,
					workspace_id as _,
				)
				.fetch_optional(&mut **database)
				.await?
				.ok_or(ErrorType::WrongParameters)?;

				runner_id_to_update = Some(deployment.runner);

				url_type = Some(ManagedUrlTypeDiscriminant::ProxyDeployment);
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
				url_type = Some(ManagedUrlTypeDiscriminant::ProxyStaticSite);
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
				url_type = Some(ManagedUrlTypeDiscriminant::ProxyUrl);
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
				url_type = Some(ManagedUrlTypeDiscriminant::Redirect);
				deployment_id = None;
				port = None;
				static_site_id = None;
				url = Some(managed_url_url);
				permanent_redirect = Some(managed_url_permanent_redirect);
				http_only = Some(managed_url_http_only);
			}
		}
	} else {
		url_type = None;
		deployment_id = None;
		port = None;
		static_site_id = None;
		url = None;
		permanent_redirect = None;
		http_only = None;
	}

	query!(
		r#"
		UPDATE
			managed_url
		SET
			path = COALESCE($2, path),
			url_type = COALESCE($3, url_type),
			deployment_id = CASE WHEN $3 IS NOT NULL
				THEN $4
				ELSE deployment_id
			END,
			port = CASE WHEN $3 IS NOT NULL
				THEN $5
				ELSE port
			END,
			static_site_id = CASE WHEN $3 IS NOT NULL
				THEN $6
				ELSE static_site_id
			END,
			url = CASE WHEN $3 IS NOT NULL
				THEN $7
				ELSE url
			END,
			permanent_redirect = CASE WHEN $3 IS NOT NULL
				THEN $8
				ELSE permanent_redirect
			END,
			http_only = CASE WHEN $3 IS NOT NULL
				THEN $9
				ELSE http_only
			END
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

	super::sync_worker_kv_for_domain(
		&format!("{}.{}", managed_url.sub_domain, managed_url.domain),
		database,
		&state.config,
	)
	.await?;

	if let Some(runner_id) = runner_id_to_update {
		super::super::runner::update_cloudflare_config_for_runner(
			runner_id,
			database,
			&state.config,
		)
		.await?;
	}

	AppResponse::builder()
		.body(UpdateManagedURLResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
