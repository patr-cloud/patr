use axum::http::StatusCode;
use models::{api::workspace::infrastructure::managed_url::*, prelude::*};

use crate::prelude::*;

pub async fn create_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateManagedURLPath { workspace_id },
				query: (),
				headers:
					CreateManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateManagedURLRequestProcessed {
						sub_domain,
						domain_id,
						path,
						url_type: managed_url_type,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, CreateManagedURLRequest>,
) -> Result<AppResponse<CreateManagedURLRequest>, ErrorType> {
	info!(
		"Creating ManagedURL with sub_domain: `{}`, domain_id: `{}` and path: `{}`",
		sub_domain, domain_id, path
	);

	let domain = query!(
		r#"
		SELECT
			workspace_domain.name,
			workspace_domain.tld
		FROM
			workspace_domain
		INNER JOIN
			resource
		ON
			workspace_domain.id = resource.id
		WHERE
			workspace_domain.id = $1 AND
			workspace_domain.deleted IS NULL AND
			resource.owner_id = $2;
		"#,
		domain_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::WrongParameters)?;

	let domain = format!("{}.{}", domain.name, domain.tld);
	let path = format!("/{}", path.trim_start_matches('/'));

	info!("Creating ManagedURL: `{}.{}{}`", sub_domain, domain, path);

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

	let id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'managed_url'),
				$1,
				NOW()
			)
		RETURNING id;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?
	.id;

	query!(
		r#"
		INSERT INTO
			managed_url(
				id,
				sub_domain,
				domain_id,
				path,
				url_type,
				deployment_id,
				port,
				static_site_id,
				url,
				workspace_id,
				is_configured,
				deleted,
				permanent_redirect,
				http_only
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				$9,
				$10,
				FALSE,
				NULL,
				$11,
				$12
			);
		"#,
		id as _,
		sub_domain,
		domain_id as _,
		path,
		url_type as _,
		deployment_id as _,
		port.map(|port| port as i32),
		static_site_id as _,
		url,
		workspace_id as _,
		permanent_redirect,
		http_only,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateManagedURLResponse {
			id: WithId::from(id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
