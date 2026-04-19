use std::fs;

use models::{
	api::workspace::{container_registry::*, deployment::*, runner::*},
	iaac::*,
	utils::BearerToken,
};

use crate::prelude::*;

/// Apply an IaaC deployment resource — creating the deployment on Patr and
/// associating it with a runner + registry image.
pub async fn apply(
	workspace_id: Uuid,
	token: BearerToken,
	IaacDeployment {
		id,
		name,
		image,
		runner,
		machine_type: _,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		ports,
		environment_variables,
		startup_probe,
		liveness_probe,
		config_mounts,
	}: IaacDeployment,
) -> Result<(), AppError> {
	let (registry, image_tag) = match image.resolve_value()? {
		IaacDeploymentImage::PatrRegistry {
			registry,
			repository,
			tag,
		} => {
			let repository_id = make_request(
				ApiRequest::<ListContainerRepositoriesRequest>::builder()
					.path(ListContainerRepositoriesPath { workspace_id })
					.headers(ListContainerRepositoriesRequestHeaders {
						authorization: token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.query(ListResourceQuery {
						search: ContainerRepositorySearchParams {
							name: Some(repository.clone()),
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
			.repositories
			.into_iter()
			.find(|repo| repo.name == repository)
			.map(|repo| repo.id)
			.ok_or_else(|| {
				AppError::IaacError(IaacError::ResourceNotFound(format!(
					"Container repository '{repository}' not found in workspace",
				)))
			})?;
			(
				DeploymentRegistry::PatrRegistry {
					registry,
					repository_id,
				},
				tag,
			)
		}
		IaacDeploymentImage::ExternalRegistry {
			registry,
			repository,
			tag,
		} => (
			DeploymentRegistry::ExternalRegistry {
				registry,
				image_name: repository,
			},
			tag,
		),
	};

	let runner = runner.resolve_value()?;
	let runner = make_request(
		ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.headers(ListRunnersForWorkspaceRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				search: RunnerSearchParams {
					name: Some(runner.clone()),
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
	.runners
	.into_iter()
	.find(|r| r.name == runner)
	.map(|runner| runner.id)
	.ok_or_else(|| {
		AppError::IaacError(IaacError::ResourceNotFound(format!(
			"Runner '{runner}' not found in workspace"
		)))
	})?;

	let machine_type = make_request(
		ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
			.path(ListAllDeploymentMachineTypePath { workspace_id })
			.headers(ListAllDeploymentMachineTypeRequestHeaders {
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.machine_types
	.into_iter()
	.next()
	.ok_or_else(|| {
		AppError::IaacError(IaacError::ResourceNotFound(
			"No deployment machine types found in workspace".to_string(),
		))
	})?
	.id;

	let name = name.resolve_value()?;

	let deployment_id = make_request(
		ApiRequest::<ListDeploymentRequest>::builder()
			.path(ListDeploymentPath { workspace_id })
			.headers(ListDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				search: DeploymentSearchParams {
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
	.deployments
	.into_iter()
	.find(|d| d.name == name)
	.map(|d| d.id);

	// If an ID is provided, specifically use that. Otherwise, use the found
	// deployment ID by name.
	if let Some(deployment_id) = id.or(deployment_id) {
		eprintln!("Updating existing deployment `{name}` with ID `{deployment_id}`");

		make_request(
			ApiRequest::<UpdateDeploymentRequest>::builder()
				.path(UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.headers(UpdateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.body(UpdateDeploymentRequest {
					name: Some(name.clone()),
					runner: Some(runner),
					machine_type: Some(machine_type),
					deploy_on_push: Some(deploy_on_push.resolve_value()?),
					min_horizontal_scale: Some(min_horizontal_scale.resolve_value()?),
					max_horizontal_scale: Some(max_horizontal_scale.resolve_value()?),
					ports: Some(ports.into_inner()),
					environment_variables: Some(
						environment_variables
							.into_inner()
							.into_iter()
							.map(|(key, value)| Ok((key, value.resolve_value()?)))
							.collect::<Result<_, IaacError>>()?,
					),
					startup_probe,
					liveness_probe,
					config_mounts: Some(
						config_mounts
							.into_iter()
							.map(|(key, mount)| {
								(
									key,
									Base64String::from_string(
										fs::read_to_string(&mount).unwrap_or(mount),
									),
								)
							})
							.collect(),
					),
					volumes: Default::default(),
				})
				.build(),
		)
		.await?;

		eprintln!("Deployment `{name}` (with ID `{deployment_id}`) updated");
	} else {
		// If no ID is provided and no deployment is found by name, create a new
		// deployment.
		eprintln!("Creating new deployment `{name}`");

		let response = make_request(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id })
				.headers(CreateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: name.clone(),
					registry,
					image_tag,
					runner,
					machine_type,
					running_details: DeploymentRunningDetails {
						deploy_on_push: deploy_on_push.resolve_value()?,
						min_horizontal_scale: min_horizontal_scale.resolve_value()?,
						max_horizontal_scale: max_horizontal_scale.resolve_value()?,
						ports: ports.into_inner(),
						environment_variables: environment_variables
							.into_inner()
							.into_iter()
							.map(|(key, value)| Ok((key, value.resolve_value()?)))
							.collect::<Result<_, IaacError>>()?,
						startup_probe,
						liveness_probe,
						config_mounts: config_mounts
							.into_iter()
							.map(|(key, mount)| {
								(
									key,
									Base64String::from_string(
										fs::read_to_string(&mount).unwrap_or(mount),
									),
								)
							})
							.collect(),
						volumes: Default::default(),
					},
					deploy_on_create: true,
				})
				.build(),
		)
		.await?;

		eprintln!(
			"Deployment `{name}` created with ID `{}`",
			response.body.id.id
		);
	}

	Ok(())
}
