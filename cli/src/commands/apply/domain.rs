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
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.query(ListResourceQuery {
				search: Default::default(),
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
		println!("Domain `{name}` with ID `{domain_id}` already exists");
		println!(
			"Note: Domains cannot be updated once created. To modify a domain, delete and recreate it."
		);
	} else {
		// If no ID is provided and no domain is found by name, create a new domain.
		println!("Creating new domain `{name}`");

		let response = make_request(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath { workspace_id })
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.body(AddDomainToWorkspaceRequest {
					domain: name.clone(),
					nameserver_type: nameserver_type.clone(),
				})
				.build(),
		)
		.await?;

		println!("Domain `{name}` created with ID `{}`", response.body.id.id);

		// If the nameserver is external, provide verification instructions
		if nameserver_type.is_external() {
			println!(
				"\nTo verify this domain, you need to add the verification records to your DNS provider."
			);
			println!(
				"Run `patr domain verify {}` for verification instructions.",
				response.body.id.id
			);
		}
	}

	Ok(())
}
