use axum::http::StatusCode;
use models::{
	api::workspace::{managed_url::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	prelude::*,
};
use rustis::commands::PubSubCommands;

use crate::prelude::*;

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
		redis,
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
			managed_url.domain_id AS "domain_id: Uuid",
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain!",
			managed_url.path,
			managed_url.url_type AS "url_type: ManagedUrlTypeDiscriminant",
			managed_url.deployment_id AS "deployment_id: Uuid",
			managed_url.port,
			deployment.runner AS "deployment_runner?: Uuid",
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
		managed_url_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	if !managed_url.is_verified {
		return Err(ErrorType::DomainNotVerified);
	}

	// Capture the old ProxyDeployment state (deployment + runner) so we can
	// emit a delete to the old runner if the URL is moving away or repointing.
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
	let static_site_id;
	let url;
	let permanent_redirect;
	let http_only;

	let mut new_proxy_deployment = None::<(Uuid, Uuid)>;
	let mut new_url_type_payload = None::<ManagedUrlType>;

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

			new_proxy_deployment = Some((deployment.runner, managed_url_deployment_id));
			new_url_type_payload = Some(ManagedUrlType::ProxyDeployment {
				deployment_id: managed_url_deployment_id,
				port: managed_url_port,
			});

			url_type = ManagedUrlTypeDiscriminant::ProxyDeployment;
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
			url_type = ManagedUrlTypeDiscriminant::ProxyStaticSite;
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
			url_type = ManagedUrlTypeDiscriminant::ProxyUrl;
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
			url_type = ManagedUrlTypeDiscriminant::Redirect;
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

	let new_runner = new_proxy_deployment.map(|(r, _)| r);

	// Build the stream payload for create/update messages, if applicable.
	let new_managed_url_payload = new_url_type_payload.map(|url_type| {
		WithId::new(
			managed_url_id,
			ManagedUrl {
				sub_domain: managed_url.sub_domain.clone(),
				domain_id: managed_url.domain_id,
				path: path.clone(),
				url_type,
				is_active: false,
			},
		)
	});

	let old_runner = old_proxy_deployment.map(|(r, _)| r);
	if old_runner == new_runner && new_runner.is_some() {
		if let Some(payload) = new_managed_url_payload {
			redis
				.publish(
					format!("{}/runner/{}/stream", workspace_id, new_runner.unwrap()),
					serde_json::to_string(
						&StreamRunnerDataForWorkspaceServerMsg::ManagedUrlUpdated {
							managed_url: payload,
						},
					)
					.unwrap(),
				)
				.await?;
		}
	} else {
		if let Some(r) = old_runner {
			redis
				.publish(
					format!("{}/runner/{}/stream", workspace_id, r),
					serde_json::to_string(
						&StreamRunnerDataForWorkspaceServerMsg::ManagedUrlDeleted {
							id: managed_url_id,
						},
					)
					.unwrap(),
				)
				.await?;
		}
		if let (Some(r), Some(payload)) = (new_runner, new_managed_url_payload) {
			redis
				.publish(
					format!("{}/runner/{}/stream", workspace_id, r),
					serde_json::to_string(
						&StreamRunnerDataForWorkspaceServerMsg::ManagedUrlCreated {
							managed_url: payload,
						},
					)
					.unwrap(),
				)
				.await?;
		}
	}

	AppResponse::builder()
		.body(UpdateManagedURLResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
