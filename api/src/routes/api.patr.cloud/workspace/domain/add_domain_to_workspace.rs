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
				body: AddDomainToWorkspaceRequestProcessed { domain },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, AddDomainToWorkspaceRequest>,
) -> Result<AppResponse<AddDomainToWorkspaceRequest>, ErrorType> {
	info!("Adding domain `{domain}` to workspace `{workspace_id}`");

	let now = OffsetDateTime::now_utc();

	let suffix = psl::Psl::suffix(&psl::List, domain.as_bytes())
		.ok_or(ErrorType::NotRootDomain)?
		.trim();
	let tld = String::from_utf8_lossy(suffix.as_bytes());
	let name = domain.trim_end_matches(&format!(".{tld}"));

	if suffix.typ() != Some(psl::Type::Icann) {
		return Err(ErrorType::NotIcannDomain);
	}

	if name.contains('.') {
		return Err(ErrorType::NotRootDomain);
	}

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
				is_verified,
				deleted
			)
		VALUES
			($1, $2, $3, $4, FALSE, NULL);
		"#,
		domain_id as _,
		name as _,
		tld as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => ErrorType::server_error(err),
	})?;

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
