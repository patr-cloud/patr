use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{
	api::workspace::{managed_url::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	prelude::*,
};
use rustis::commands::PubSubCommands;

use crate::prelude::*;

/// The handler to create a new managed URL in a workspace. This will create a
/// new managed URL with the provided subdomain, domain, and path. The URL type
/// can be a proxy to a deployment, a proxy to a static site, a proxy to a URL,
/// or a redirect to a URL. The URL type will determine how the managed URL
/// behaves.
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
		redis,
		client_ip: _,
		user_data: _,
		state,
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
			workspace_domain.tld,
			workspace_domain.is_verified
		FROM
			workspace_domain
		INNER JOIN
			resource
		ON
			workspace_domain.id = resource.id
		WHERE
			workspace_domain.id = $1 AND
			workspace_domain.deleted IS NULL AND
			resource.workspace_id = $2;
		"#,
		domain_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::WrongParameters)?;

	if !domain.is_verified {
		return Err(ErrorType::DomainNotVerified);
	}

	let domain = format!("{}.{}", domain.name, domain.tld);
	let path = format!("/{}", path.trim_start_matches('/'));

	info!("Creating ManagedURL: `{}.{}{}`", sub_domain, domain, path);

	// TODO: Check if the user has access to the deployment or static site (ON THE
	// RIGHT WORKSPACE) if the URL type is a proxy.
	let mut deployment_runner = None::<Uuid>;
	let (url_discriminant, deployment_id, port, static_site_id, url, permanent_redirect, http_only) =
		match url_type.clone() {
			ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
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
						resource.workspace_id = $2;
					"#,
					deployment_id as _,
					workspace_id as _,
				)
				.fetch_optional(&mut **database)
				.await?
				.ok_or(ErrorType::WrongParameters)?;

				deployment_runner = Some(deployment.runner);

				(
					ManagedUrlTypeDiscriminant::ProxyDeployment,
					Some(deployment_id),
					Some(port),
					None,
					None,
					None,
					None,
				)
			}
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

	// Ensure a Cloudflare Custom Hostname exists for this FQDN before
	// inserting the managed URL (FK requires the custom hostname row to exist)
	let existing_custom_hostname = query!(
		r#"
		SELECT
			cloudflare_custom_hostname_id
		FROM
			managed_url_custom_hostname
		WHERE
			sub_domain = $1 AND
			domain_id = $2
		FOR UPDATE;
		"#,
		&sub_domain,
		domain_id as _,
	)
	.fetch_optional(&mut **database)
	.await?;

	if existing_custom_hostname.is_none() {
		cfg_if! {
			if #[cfg(feature = "cloud")] {
				let fqdn = if sub_domain == "@" {
					domain.clone()
				} else {
					format!("{}.{}", sub_domain, domain)
				};

				let cf_client = CloudflareClient::new(
					Credentials::UserAuthToken {
						token: state.config.cloudflare.api_key.clone(),
					},
					ClientConfig::default(),
					Environment::Custom(state.config.cloudflare.base_url.clone()),
				)?;

				let custom_hostname_id = cf_client
					.request(&AddCustomHostname {
						zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
						params: AddCustomHostnameParams {
							hostname: fqdn,
							ssl: Some(CustomHostnameSsl {
								bundle_method: Some(CustomHostnameSslBundleMethod::Ubiquitous),
								certificate_authority: Some(
									CustomHostnameSslCertificateAuthority::LetsEncrypt,
								),
								type_: Some(CustomHostnameSslType::DV),
								method: Some(CustomHostnameSslMethod::Http),
								validation_records: None,
								settings: None,
								wildcard: None,
								status: None,
							}),
							custom_metadata: None,
						},
					})
					.await?
					.result
					.id;

				query!(
					r#"
					INSERT INTO
						managed_url_custom_hostname(
							sub_domain,
							domain_id,
							cloudflare_custom_hostname_id,
							is_active
						)
					VALUES
						($1, $2, $3, FALSE)
					ON CONFLICT (sub_domain, domain_id)
					DO NOTHING;
					"#,
					&sub_domain,
					domain_id as _,
					&custom_hostname_id,
				)
				.execute(&mut **database)
				.await?;
			} else {
				let _ = state;
				query!(
					r#"
					INSERT INTO
						managed_url_custom_hostname(
							sub_domain,
							domain_id,
							cloudflare_custom_hostname_id,
							is_active
						)
					VALUES
						($1, $2, '', TRUE)
					ON CONFLICT (sub_domain, domain_id)
					DO NOTHING;
					"#,
					&sub_domain,
					domain_id as _,
				)
				.execute(&mut **database)
				.await?;
			}
		}
	}

	let id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				workspace_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'managedUrl'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
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
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
			ErrorType::ResourceAlreadyExists
		}
		err => ErrorType::server_error(err),
	})?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			utils::cloudflare::sync_ingress_kv_for_fqdn(
				&format!("{}.{}", sub_domain, domain),
				database,
				&state.config,
			)
			.await?;
			let is_active = false;
		} else {
			let _ = (database, &state, &domain);
			let is_active = true;
		}
	}

	// Notify the runner that owns the target deployment, if this URL targets
	// one. Other URL types stay Cloudflare-only for now.
	if let Some(runner_id) = deployment_runner {
		redis
			.publish(
				format!("{}/runner/{}/stream", workspace_id, runner_id),
				serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::ManagedUrlCreated {
					managed_url: WithId::new(
						id,
						ManagedUrl {
							sub_domain: sub_domain.to_string(),
							domain_id,
							path: path.clone(),
							url_type: url_type.clone(),
							is_active,
						},
					),
				})
				.unwrap(),
			)
			.await?;
	}

	AppResponse::builder()
		.body(CreateManagedURLResponse {
			id: WithId::from(id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
