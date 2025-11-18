use axum::http::StatusCode;
use cloudflare::{
	endpoints::workerskv::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to create a new managed URL in a workspace. This will create a
/// new managed URL with the provided subdomain, domain, and path. The URL type
/// can be a proxy to a deployment, a proxy to a static site, a proxy to a URL,
/// or a redirect to a URL. The URL type will determine how the managed URL
/// behaves.
#[instrument(skip(database, config))]
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
						url_type,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config,
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

	let (url_discriminant, deployment_id, port, static_site_id, url, permanent_redirect, http_only) =
		match url_type.clone() {
			ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			} => (
				ManagedUrlTypeDiscriminant::ProxyDeployment,
				Some(deployment_id),
				Some(port),
				None,
				None,
				None,
				None,
			),
			ManagedUrlType::ProxyStaticSite { static_site_id } => (
				ManagedUrlTypeDiscriminant::ProxyStaticSite,
				None,
				None,
				Some(static_site_id),
				None,
				None,
				None,
			),
			ManagedUrlType::ProxyUrl {
				url: managed_url_url,
				http_only: managed_url_http_only,
			} => (
				ManagedUrlTypeDiscriminant::ProxyUrl,
				None,
				None,
				None,
				Some(managed_url_url),
				None,
				Some(managed_url_http_only),
			),
			ManagedUrlType::Redirect {
				url: managed_url_url,
				permanent_redirect: managed_url_permanent_redirect,
				http_only: managed_url_http_only,
			} => (
				ManagedUrlTypeDiscriminant::Redirect,
				None,
				None,
				None,
				Some(managed_url_url),
				Some(managed_url_permanent_redirect),
				Some(managed_url_http_only),
			),
		};

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
		err => ErrorType::server_error(err),
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
		&sub_domain,
		domain_id as _,
		path,
		url_discriminant as _,
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

	let client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?;

	use models::cloudflare::kv::WorkerManagedUrlKVValue::*;

	let kv_body = match url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => {
			let runner_id = query!(
				r#"
				SELECT
					runner AS "runner: Uuid"
				FROM
					deployment
				WHERE
					id = $1;
				"#,
				deployment_id as _,
			)
			.fetch_one(&mut **database)
			.await?
			.runner;

			ProxyDeployment {
				deployment_id,
				port,
				runner_id,
			}
		}
		ManagedUrlType::ProxyStaticSite { static_site_id } => ProxyStaticSite { static_site_id },
		ManagedUrlType::ProxyUrl { url, http_only } => ProxyUrl { url, http_only },
		ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => Redirect {
			url,
			permanent_redirect,
			http_only,
		},
	};

	client
		.request(&write_key::WriteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.worker_namespace_id,
			key: &format!(
				"{}.{}{}",
				sub_domain,
				domain,
				if path == "/" { "" } else { &path }
			),
			params: write_key::WriteKeyParams {
				expiration: None,
				expiration_ttl: None,
			},
			body: write_key::WriteKeyBody::Value(serde_json::to_vec(&kv_body)?),
		})
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
