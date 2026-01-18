use models::{
	api::workspace::{deployment::*, domain::*, managed_url::*},
	iaac::*,
	utils::{BearerToken, ResourceSearcher},
};

use crate::prelude::*;

pub async fn apply(
	workspace_id: Uuid,
	token: BearerToken,
	IaacManagedUrl {
		id,
		sub_domain,
		domain,
		path,
		to,
	}: IaacManagedUrl,
) -> Result<(), AppError> {
	let sub_domain = sub_domain.resolve_value()?;
	let domain = domain.resolve_value()?;
	let path = path.resolve_value()?;

	// Resolve domain name to domain ID
	let domain_id = make_request(
		ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
			.path(ListDomainsInWorkspacePath { workspace_id })
			.headers(ListDomainsInWorkspaceRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				search: WorkspaceDomainSearchParams {
					name: Some(domain.clone()),
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
	.domains
	.into_iter()
	.find(|d| d.name == domain)
	.map(|d| d.id)
	.ok_or_else(|| {
		AppError::IaacError(IaacError::ResourceNotFound(format!(
			"Domain '{domain}' not found in workspace",
		)))
	})?;

	// Resolve IaacManagedUrlType to API ManagedUrlType
	let url_type = match to {
		IaacManagedUrlType::ProxyDeployment { deployment, port } => {
			// Find deployment by name
			let deployment_id = make_request(
				ApiRequest::<ListDeploymentRequest>::builder()
					.path(ListDeploymentPath { workspace_id })
					.headers(ListDeploymentRequestHeaders {
						authorization: token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.query(ListResourceQuery {
						search: DeploymentSearchParams {
							name: Some(deployment.clone()),
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
			.deployments
			.into_iter()
			.find(|d| d.name == deployment)
			.map(|d| d.id)
			.ok_or_else(|| {
				AppError::IaacError(IaacError::ResourceNotFound(format!(
					"Deployment '{deployment}' not found in workspace",
				)))
			})?;

			let ports = make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id,
						deployment_id,
					})
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.build(),
			)
			.await?
			.body
			.running_details
			.ports;

			let port = ports
				.iter()
				.filter(|(_, port_type)| matches!(port_type, ExposedPortType::Http))
				.filter(|(exposed_port, _)| *exposed_port.as_ref() == port)
				.next()
				.ok_or_else(|| {
					AppError::IaacError(IaacError::ResourceNotFound(format!(
						concat!(
							"Error while applying the Managed URL `{}`, ",
							"you have chosen port `{}` for deployment `{}`, ",
							"but the deployment `{}` doesn't expose that port. ",
							"Available ports are: [{}]"
						),
						if sub_domain == "@" {
							format!("{sub_domain}.{domain}/{path}")
						} else {
							format!("{domain}/{path}")
						},
						port,
						deployment,
						deployment,
						ports
							.keys()
							.map(ToString::to_string)
							.collect::<Vec<_>>()
							.join(", ")
					)))
				})?
				.0
				.value();

			ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			}
		}
		IaacManagedUrlType::ProxyStaticSite { static_site: _ } => {
			// Find static site by name

			return Err(AppError::IaacError(IaacError::Unsupported(format!(
				"Static sites are not supported yet"
			))));
		}
		IaacManagedUrlType::ProxyUrl { url, http_only } => {
			ManagedUrlType::ProxyUrl { url, http_only }
		}
		IaacManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		},
	};

	// Check if managed URL already exists
	let managed_url_id = make_request(
		ApiRequest::<ListManagedURLRequest>::builder()
			.path(ListManagedURLPath { workspace_id })
			.headers(ListManagedURLRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				search: ManagedUrlSearchParams {
					sub_domain: Some(sub_domain.clone()),
					domain_id: Some(ResourceSearcher {
						resource_id: domain_id,
					}),
					path: Some(path.clone()),
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
	.urls
	.into_iter()
	.find(|u| u.sub_domain == sub_domain && u.domain_id == domain_id && u.path == path)
	.map(|u| u.id);

	// If an ID is provided, specifically use that. Otherwise, use the found
	// managed URL ID by subdomain and domain.
	if let Some(managed_url_id) = id.or(managed_url_id) {
		eprintln!(
			"Updating existing managed URL `{}.{}/{}` with ID `{}`",
			sub_domain, domain, path, managed_url_id
		);

		make_request(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id,
					managed_url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: Some(path.clone()),
					url_type: Some(url_type),
				})
				.build(),
		)
		.await?;

		eprintln!(
			"Managed URL `{}.{}{}` (with ID `{}`) updated",
			sub_domain, domain, path, managed_url_id
		);
	} else {
		// If no ID is provided and no managed URL is found, create a new one.
		eprintln!(
			"Creating new managed URL `{}.{}/{}`",
			sub_domain, domain, path
		);

		let response = make_request(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath { workspace_id })
				.headers(CreateManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: sub_domain.clone(),
					domain_id,
					path: path.clone(),
					url_type,
				})
				.build(),
		)
		.await?;

		eprintln!(
			"Managed URL `{}.{}/{}` created with ID `{}`",
			sub_domain, domain, path, response.body.id.id
		);
	}

	Ok(())
}
