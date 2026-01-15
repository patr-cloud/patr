use cloudflare::{
	endpoints::{
		workers::*,
		zones::{
			custom_hostnames::*,
			zone::{Type as ZoneType, *},
		},
	},
	framework::{
		Environment,
		SearchMatch,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::api::workspace::domain::*;
use reqwest::StatusCode;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn add_domain_to_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: AddDomainToWorkspacePath { workspace_id },
				query: (),
				headers:
					AddDomainToWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					AddDomainToWorkspaceRequestProcessed {
						domain,
						nameserver_type,
					},
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, AddDomainToWorkspaceRequest>,
) -> Result<AppResponse<AddDomainToWorkspaceRequest>, ErrorType> {
	info!("Adding domain `{domain}` to workspace `{workspace_id}`");

	let now = OffsetDateTime::now_utc();

	let domain_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created,
				deleted
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'domain'),
				$1,
				$2,
				NULL
			)
		RETURNING id;
		"#,
		workspace_id as _,
		now as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.id;

	let suffix = psl::Psl::suffix(&psl::List, domain.as_bytes())
		.ok_or(ErrorType::NotRootDomain)?
		.trim();
	let tld = String::from_utf8_lossy(suffix.as_bytes());
	let name = domain.trim_end_matches(&format!(".{tld}"));

	if suffix.typ() != Some(psl::Type::Icann) {
		return Err(ErrorType::NotIcannDomain);
	}

	let contains_dot = name.contains('.');
	if contains_dot {
		return Err(ErrorType::NotRootDomain);
	}

	query!(
		r#"
		INSERT INTO
			domain_tld
		VALUES
			($1)
		ON CONFLICT DO NOTHING;
		"#,
		tld as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			workspace_domain(
				id,
                name,
                tld,
                workspace_id,
                nameserver_type,
				cloudflare_worker_route_id,
				cloudflare_custom_hostname_id,
                is_verified,
                deleted
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				'undefined',
				'undefined',
				FALSE,
				NULL
			);
		"#,
		domain_id as _,
		name as _,
		tld as _,
		workspace_id as _,
		nameserver_type as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => ErrorType::server_error(err),
	})?;

	let client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?;

	let worker_route_id = client
		.request(&CreateRoute {
			zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
			params: CreateRouteParams {
				pattern: format!("*.{domain}/*"),
				script: Some(state.config.cloudflare.ingress_script_name.clone()),
			},
		})
		.await?
		.result
		.id;

	let custom_hostname_id = client
		.request(&AddCustomHostname {
			zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
			params: AddCustomHostnameParams {
				hostname: domain.clone(),
				ssl: Some(CustomHostnameSsl {
					bundle_method: Some(CustomHostnameSslBundleMethod::Ubiquitous),
					certificate_authority: Some(CustomHostnameSslCertificateAuthority::Google),
					type_: Some(CustomHostnameSslType::DV),
					method: Some(CustomHostnameSslMethod::Txt),
					validation_records: None,
					settings: None,
					wildcard: Some(true),
				}),
				custom_metadata: None,
			},
		})
		.await?
		.result
		.id;

	query!(
		r#"
		UPDATE
			workspace_domain
		SET
			cloudflare_worker_route_id = $2,
			cloudflare_custom_hostname_id = $3
		WHERE
			id = $1;
		"#,
		domain_id as _,
		worker_route_id,
		custom_hostname_id,
	)
	.execute(&mut **database)
	.await?;

	match nameserver_type {
		DomainNameserverType::Internal => {
			print!("Internal nameservers are not yet supported.");
			return Err(ErrorType::InternalServerError);
		}
		DomainNameserverType::External => {
			query!(
				r#"
                INSERT INTO
                    user_controlled_domain(
                        domain_id,
                        nameserver_type
                    )
                VALUES
                    (
                        $1,
                        $2
                    );
                "#,
				domain_id as _,
				nameserver_type as _,
			)
			.execute(&mut **database)
			.await
			.map_err(|err| match err {
				sqlx::Error::Database(err) if err.is_unique_violation() => {
					ErrorType::ResourceAlreadyExists
				}
				err => ErrorType::server_error(err),
			})?;
		}
	}

	trace!("Created domain with ID: {}", domain_id);

	AppResponse::builder()
		.body(AddDomainToWorkspaceResponse {
			id: WithId::from(domain_id),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
