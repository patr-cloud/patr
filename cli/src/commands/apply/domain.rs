use models::{api::workspace::domain::*, iaac::*, utils::BearerToken};

use crate::prelude::*;

pub async fn apply(
	workspace_id: Uuid,
	token: BearerToken,
	IaacDomain {
		id,
		name,
		nameserver_type,
	}: IaacDomain,
) -> Result<(), AppError> {
	let name = name.resolve_value()?;
	let nameserver_type = nameserver_type.resolve_value()?;

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

	// Filter for exact match (case-insensitive) locally
	let domain_id = domains
		.into_iter()
		.find(|d| d.name.eq_ignore_ascii_case(&name))
		.map(|d| d.id);

	// If an ID is provided, specifically use that. Otherwise, use the found
	// domain ID by name.
	if let Some(domain_id) = id.or(domain_id) {
		eprintln!("Domain `{name}` with ID `{domain_id}` already exists");
		eprintln!(
			"Note: Domains cannot be updated once created. To modify a domain, delete and recreate it."
		);

		return Ok(());
	} else {
		// If no ID is provided and no domain is found by name, create a new domain.
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
					nameserver_type: nameserver_type.clone(),
				})
				.build(),
		)
		.await?;

		eprintln!("Domain `{name}` created with ID `{}`", response.body.id.id);

		// If the nameserver is external, provide verification instructions
		if nameserver_type.is_external() {
			eprintln!(
				"To verify this domain, you need to add the verification records to your DNS provider."
			);
			eprintln!("Run `patr domain verify {name}` for verification instructions.",);
		} else {
			eprintln!("Internal domains are currently not supported.")
		}
	}

	Ok(())
}
