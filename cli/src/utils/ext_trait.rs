use std::{collections::BTreeMap, fmt::Display};

use inquire::InquireError;
use models::{
	api::workspace::{container_registry::*, deployment::*, runner::*},
	iaac::*,
};

use crate::prelude::*;

/// Trait to extend the `Result` type with a method to handle TTY expectations.
/// This trait provides a method to handle the case where the terminal is not a
/// TTY when expecting user input.
pub trait TtyExpectable<T> {
	/// Handles the case where the terminal is not a TTY when expecting user
	/// input. If the result is `Ok`, it returns the value. If the result is an
	/// error indicating that the terminal is not a TTY, it prints an error
	/// message and exits the process with a failure code.
	fn expect_tty(self, message: impl Display) -> T;
}

impl<T> TtyExpectable<T> for Result<T, InquireError> {
	fn expect_tty(self, message: impl Display) -> T {
		let message = message.to_string();
		match self {
			Ok(value) => value,
			Err(InquireError::NotTTY) => {
				eprintln!(concat!(
					"The terminal the CLI is running in is not a TTY. ",
					"You either need to provide a CLI flag for the value you are trying to set, ",
					"or use an interactive terminal to allow the CLI to prompt you for the value.",
				));
				std::process::ExitCode::FAILURE.exit_process();
			}
			err => err.expect(&message),
		}
	}
}

/// Trait to extend the `String` type with helper methods
pub trait StringExt {
	/// Returns a `String` if the string is not empty, otherwise returns `None`.
	/// This is useful for converting a string to an `Option<String>` based on
	/// its contents.
	fn some_if_not_empty(self) -> Option<String>;
}

impl StringExt for String {
	fn some_if_not_empty(self) -> Option<String> {
		if self.is_empty() { None } else { Some(self) }
	}
}

/// Trait to extend the `IaacResolver` type with a method to resolve the value
/// of the IaacResolver to the actual value and return it.
pub trait IaacResolverExt<T> {
	/// Resolves the value of the IaacResolver to the actual value and returns
	/// it.
	fn resolve_value(
		self,
		workspace_id: Uuid,
		api_token: BearerToken,
	) -> impl Future<Output = Result<T, AppError>>;
}

impl IaacResolverExt<CreateDeploymentRequest> for IaacDeployment {
	async fn resolve_value(
		self,
		workspace_id: Uuid,
		api_token: BearerToken,
	) -> Result<CreateDeploymentRequest, AppError> {
		let image = self.image.resolve_value()?;

		let (registry, image_tag) = match image {
			IaacDeploymentImage::PatrRegistry {
				registry,
				repository,
				tag,
			} => {
				let repository_id = make_request(
					ApiRequest::<ListContainerRepositoriesRequest>::builder()
						.path(ListContainerRepositoriesPath { workspace_id })
						.headers(ListContainerRepositoriesRequestHeaders {
							authorization: api_token.clone(),
							user_agent: constants::USER_AGENT,
						})
						.query(ListResourceQuery {
							search: ContainerRepositorySearchParams {
								name: Some(repository.clone()),
								..Default::default()
							},
							..Default::default()
						})
						.build(),
				)
				.await?
				.body
				.repositories
				.into_iter()
				.next()
				.map(|repo| repo.id)
				.ok_or_else(|| {
					AppError::IaacParseError(format!(
						"No container repository found with name: `{}`",
						repository
					))
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

		let runner = self.runner.resolve_value()?;

		let mut runners = vec![];
		let mut start = 0;

		loop {
			let response = make_request(
				ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
					.path(ListRunnersForWorkspacePath { workspace_id })
					.query(ListResourceQuery {
						page: start / ListResourceQuery::DEFAULT_PAGE_SIZE,
						count: ListResourceQuery::DEFAULT_PAGE_SIZE,
						search: Default::default(),
						sort: Default::default(),
						additional_query: (),
					})
					.headers(ListRunnersForWorkspaceRequestHeaders {
						user_agent: constants::USER_AGENT,
						authorization: api_token.clone(),
					})
					.build(),
			)
			.await?;

			start += response.body.runners.len();

			runners.extend(response.body.runners);

			if start >= response.headers.total_count.0 {
				break;
			}
		}

		let runner_id = Uuid::parse_str(&runner).ok();
		let runner = runners
			.iter()
			.find(|r| r.name == runner || runner_id.filter(|id| r.id == *id).is_some())
			.unwrap_or_else(|| panic!("No runner found with ID or name: `{}`", runner))
			.id;

		Ok(CreateDeploymentRequest {
			name: self.name.resolve_value()?,
			registry,
			image_tag,
			deploy_on_create: false,
			runner,
			// TODO how will machine type work?
			machine_type: Uuid::parse_str("0be608bc0dfd4e2a8ece90252d3c9bce").unwrap(), /* Uuid::parse_str(&self.machine_type.resolve_value()?.to_string()).unwrap(), */
			running_details: DeploymentRunningDetails {
				deploy_on_push: self.deploy_on_push.resolve_value()?,
				min_horizontal_scale: self.min_horizontal_scale.resolve_value()?,
				max_horizontal_scale: self.max_horizontal_scale.resolve_value()?,
				ports: self.ports.into_inner(),
				environment_variables: self
					.environment_variables
					.into_inner()
					.into_iter()
					.map(|(key, value)| Ok((key, value.resolve_value()?)))
					.collect::<Result<BTreeMap<_, _>, AppError>>()?,
				startup_probe: self.startup_probe,
				liveness_probe: self.liveness_probe,
				config_mounts: self
					.config_mounts
					.into_iter()
					.map(|(key, value)| (key, Base64String::from_string(value)))
					.collect(),
			},
		})
	}
}
