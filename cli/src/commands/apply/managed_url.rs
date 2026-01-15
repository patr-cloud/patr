use models::{
	api::workspace::{deployment::*, domain::*, managed_url::*, static_site::*},
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
						user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
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

			ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			}
		}
		IaacManagedUrlType::ProxyStaticSite { static_site } => {
			// Find static site by name
			let static_site_id = make_request(
				ApiRequest::<ListStaticSiteRequest>::builder()
					.path(ListStaticSitePath { workspace_id })
					.headers(ListStaticSiteRequestHeaders {
						authorization: token.clone(),
						user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
					})
					.query(ListResourceQuery {
						search: StaticSiteSearchParams {
							name: Some(static_site.clone()),
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
			.static_sites
			.into_iter()
			.find(|s| s.name == static_site)
			.map(|s| s.id)
			.ok_or_else(|| {
				AppError::IaacError(IaacError::ResourceNotFound(format!(
					"Static site '{static_site}' not found in workspace",
				)))
			})?;

			ManagedUrlType::ProxyStaticSite { static_site_id }
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
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.query(ListResourceQuery {
				search: ManagedUrlSearchParams {
					sub_domain: Some(sub_domain.clone()),
					domain_id: Some(ResourceSearcher {
						resource_id: domain_id,
					}),
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
		println!(
			"Updating existing managed URL `{}.{}{}` with ID `{}`",
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
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.body(UpdateManagedURLRequest {
					path: Some(path.clone()),
					url_type: Some(url_type),
				})
				.build(),
		)
		.await?;

		println!(
			"Managed URL `{}.{}{}` (with ID `{}`) updated",
			sub_domain, domain, path, managed_url_id
		);
	} else {
		// If no ID is provided and no managed URL is found, create a new one.
		println!(
			"Creating new managed URL `{}.{}{}`",
			sub_domain, domain, path
		);

		let response = make_request(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath { workspace_id })
				.headers(CreateManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
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

		println!(
			"Managed URL `{}.{}{}` created with ID `{}`",
			sub_domain, domain, path, response.body.id.id
		);
	}

	Ok(())
}
