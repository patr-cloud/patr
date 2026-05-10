use models::{api::workspace::domain::*, iaac::*, utils::BearerToken};

use crate::prelude::*;

/// Apply an IaaC domain resource — registering the domain on the workspace.
pub async fn apply(
	workspace_id: Uuid,
	token: BearerToken,
	IaacDomain { id, name }: IaacDomain,
) -> Result<(), AppError> {
	let name = name.resolve_value()?;

	let domains = make_request(
		ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
			.path(ListDomainsInWorkspacePath { workspace_id })
			.headers(ListDomainsInWorkspaceRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				search: WorkspaceDomainSearchParams {
					name: Some(name.clone()),
					..Default::default()
				},
				sort: Default::default(),
				count: ListResourceQuery::DEFAULT_PAGE_SIZE,
				page: 0,
				additional_query: (),
			})
			.build(),
	)
	.await?
	.body
	.domains;

	let domain_id = domains
		.into_iter()
		.find(|d| d.name.eq_ignore_ascii_case(&name))
		.map(|d| d.id);

	if let Some(domain_id) = id.or(domain_id) {
		eprintln!("Domain `{name}` with ID `{domain_id}` already exists");
		eprintln!(
			"Note: Domains cannot be updated once created. To modify a domain, delete and recreate it."
		);
		return Ok(());
	}

	eprintln!("Creating new domain `{name}`");

	let response = make_request(
		ApiRequest::<AddDomainToWorkspaceRequest>::builder()
			.path(AddDomainToWorkspacePath { workspace_id })
			.headers(AddDomainToWorkspaceRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.body(AddDomainToWorkspaceRequest {
				domain: name.clone(),
			})
			.build(),
	)
	.await?;

	eprintln!("Domain `{name}` created with ID `{}`", response.body.id.id);
	eprintln!(
		"To verify this domain, add the verification records to your DNS provider, then run `patr domain verify {name}`."
	);

	Ok(())
}
