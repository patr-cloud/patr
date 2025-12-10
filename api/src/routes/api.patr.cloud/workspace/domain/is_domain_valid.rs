use models::api::workspace::domain::*;
use reqwest::StatusCode;

use crate::prelude::*;

#[instrument(skip(database))]
pub async fn is_domain_valid(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: IsDomainValidPath { workspace_id },
				query: IsDomainValidQuery { domain },
				headers:
					IsDomainValidRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: IsDomainValidRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, IsDomainValidRequest>,
) -> Result<AppResponse<IsDomainValidRequest>, ErrorType> {
	info!("Checking if domain `{domain}` is valid in workspace `{workspace_id}`");

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

	let exists = query!(
		r#"
        SELECT
            id
        FROM
            workspace_domain
        WHERE
            workspace_id = $1
            AND CONCAT(name, '.', tld) = $2;
        "#,
		workspace_id as _,
		domain as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if exists {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	AppResponse::builder()
		.body(IsDomainValidResponse { valid: true })
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
