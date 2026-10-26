use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*, rbac::ResourceType};

use crate::{
	prelude::*,
	redis_publisher::{self, DirtyResource},
};

/// The handler to update a managed URL.
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
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, UpdateManagedURLRequest>,
) -> Result<AppResponse<UpdateManagedURLRequest>, ErrorType> {
	info!("Updating ManagedURL with ID: `{}`", managed_url_id);

	let managed_url = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.sub_domain,
			managed_url.domain_id,
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain!",
			managed_url.path,
			managed_url.url_type AS "url_type: ManagedUrlTypeDiscriminant",
			managed_url.deployment_id,
			managed_url.port,
			deployment.runner AS "deployment_runner?",
			workspace_domain.is_verified
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		LEFT JOIN
			deployment
		ON
			deployment.id = managed_url.deployment_id
		WHERE
			managed_url.id = $1 AND
			managed_url.deleted IS NULL AND
			managed_url.workspace_id = $2;
		"#,
		managed_url_id,
		workspace_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	if !managed_url.is_verified {
		return Err(ErrorType::DomainNotVerified);
	}

	// Capture the old ProxyDeployment state (deployment + runner) so the old
	// runner is told too if the URL is moving away or repointing.
	let old_proxy_deployment = if matches!(
		managed_url.url_type,
		ManagedUrlTypeDiscriminant::ProxyDeployment
	) {
		managed_url.deployment_runner.map(|runner| {
			(
				runner,
				managed_url
					.deployment_id
					.expect("ProxyDeployment row missing deployment_id"),
			)
		})
	} else {
		None
	};

	let path = format!("/{}", path.trim_start_matches('/'));

	let url_type;
	let deployment_id;
	let port;
	let url;
	let permanent_redirect;
	let http_only;

	let mut new_proxy_deployment = None::<(Uuid, Uuid)>;

	match managed_url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id: managed_url_deployment_id,
			port: managed_url_port,
		} => {
			let deployment = query!(
				r#"
				SELECT
					deployment.runner
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
				managed_url_deployment_id,
				workspace_id,
			)
			.fetch_optional(&mut **database)
			.await?
			.ok_or(ErrorType::WrongParameters)?;

			new_proxy_deployment = Some((deployment.runner, managed_url_deployment_id));

			url_type = ManagedUrlTypeDiscriminant::ProxyDeployment;
			deployment_id = Some(managed_url_deployment_id);
			port = Some(managed_url_port);
			url = None;
			permanent_redirect = None;
			http_only = None;
		}
		ManagedUrlType::ProxyUrl {
			url: managed_url_url,
			http_only: managed_url_http_only,
		} => {
			url_type = ManagedUrlTypeDiscriminant::ProxyUrl;
			deployment_id = None;
			port = None;
			url = Some(managed_url_url);
			permanent_redirect = None;
			http_only = Some(managed_url_http_only);
		}
		ManagedUrlType::Redirect {
			url: managed_url_url,
			permanent_redirect: managed_url_permanent_redirect,
			http_only: managed_url_http_only,
		} => {
			url_type = ManagedUrlTypeDiscriminant::Redirect;
			deployment_id = None;
			port = None;
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
			url = $6,
			permanent_redirect = $7,
			http_only = $8
		WHERE
			id = $1;
		"#,
		managed_url_id,
		path,
		url_type as _,
		deployment_id,
		port.map(|port| port as i32),
		url,
		permanent_redirect,
		http_only,
	)
	.execute(&mut **database)
	.await?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			utils::cloudflare::sync_ingress_kv_for_fqdn(
				&format!("{}.{}", managed_url.sub_domain, managed_url.domain),
				database,
				&state.config,
			)
			.await?;
		} else {
			let _ = &state;
		}
	}

	for runner_id in old_proxy_deployment
		.into_iter()
		.chain(new_proxy_deployment)
		.map(|(runner_id, _)| runner_id)
		.collect::<BTreeSet<_>>()
	{
		redis_publisher::mark_dirty(
			database,
			DirtyResource {
				workspace_id,
				runner_id,
				resource_type: ResourceType::ManagedURL,
				resource_id: managed_url_id,
			},
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
