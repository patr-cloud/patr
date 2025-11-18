use cloudflare::{
	endpoints::zones::zone::{Type as ZoneType, *},
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

#[instrument(skip(database, config))]
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
		config,
		user_data: _,
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

	let (name, tld) = {
		let tld = query!(
			r#"
			SELECT
				tld
			FROM
				domain_tld
			WHERE
				$1 ILIKE '%.' || tld
			ORDER BY
				LENGTH(tld) DESC
			LIMIT 1;
			"#,
			domain as _
		)
		.fetch_optional(&mut **database)
		.await?
		.ok_or_else(|| {
			info!("Failed to extract TLD from domain");
			ErrorType::WrongParameters
		})?
		.tld;

		let name = domain
			.strip_suffix(&format!(".{}", tld))
			.ok_or_else(|| ErrorType::WrongParameters)
			.inspect_err(|err| {
				info!("Failed to strip TLD from domain: {}", err);
			})?
			.to_string();

		(name, tld)
	};

	query!(
		r#"
		INSERT INTO
			workspace_domain(
				id,
                name,
                tld,
                workspace_id,
                nameserver_type,
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

	match nameserver_type {
		DomainNameserverType::Internal => {
			let client = CloudflareClient::new(
				Credentials::UserAuthToken {
					token: config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Production,
			)?;

			let zone = client
				.request(&ListZones {
					params: ListZonesParams {
						name: Some(domain.clone()),
						search_match: Some(SearchMatch::Any),
						..Default::default()
					},
				})
				.await?
				.result
				.into_iter()
				.next();

			if zone.is_none() {
				client
					.request(&CreateZone {
						params: CreateZoneParams {
							name: &domain,
							account: &config.cloudflare.account_id,
							jump_start: None,
							zone_type: Some(ZoneType::Full),
						},
					})
					.await?;
			}

			let zone_identifier = client
				.request(&ListZones {
					params: ListZonesParams {
						name: Some(domain.clone()),
						search_match: Some(SearchMatch::Any),
						..Default::default()
					},
				})
				.await?
				.result
				.into_iter()
				.next()
				.ok_or_else(|| {
					ErrorType::server_error("Failed to create or retrieve zone from Cloudflare")
				})?
				.id;

			query!(
				r#"
                INSERT INTO
                    patr_controlled_domain(
                        domain_id,
                        zone_identifier,
                        nameserver_type
                    )
                VALUES
                    (
                        $1,
                        $2,
                        $3
                    );
                "#,
				domain_id as _,
				zone_identifier,
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
