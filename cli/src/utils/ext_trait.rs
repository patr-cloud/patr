use std::{collections::BTreeMap, fmt::Display};

use either::Either;
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
				let repository_id = match repository {
					Either::Left(id) => id,
					Either::Right(name) => {
						let mut repositories = vec![];
						let mut start = 0;

						loop {
							let response = make_request(
								ApiRequest::<ListContainerRepositoriesRequest>::builder()
									.path(ListContainerRepositoriesPath { workspace_id })
									.headers(ListContainerRepositoriesRequestHeaders {
										authorization: api_token.clone(),
										user_agent: UserAgent::from_static(
											constants::USER_AGENT_STRING,
										),
									})
									.query(Paginated {
										page: start / Paginated::DEFAULT_PAGE_SIZE,
										count: Paginated::DEFAULT_PAGE_SIZE,
										data: (),
									})
									.body(ListContainerRepositoriesRequest)
									.build(),
							)
							.await?;

							start += response.body.repositories.len();

							repositories.extend(response.body.repositories);

							if start >= response.headers.total_count.0 {
								break;
							}
						}

						let id = Uuid::parse_str(&name).ok();
						repositories
							.iter()
							.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
							.map(|repo| repo.id)
							.ok_or_else(|| {
								AppError::IaacParseError(format!(
									"No container repository found with ID or name: `{}`",
									name
								))
							})?
					}
				};

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
					.query(Paginated {
						page: start / Paginated::DEFAULT_PAGE_SIZE,
						count: Paginated::DEFAULT_PAGE_SIZE,
						data: (),
					})
					.headers(ListRunnersForWorkspaceRequestHeaders {
						user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
						authorization: api_token.clone(),
					})
					.body(ListRunnersForWorkspaceRequest)
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
			.expect(&format!("No runner found with ID or name: `{}`", runner))
			.id;

		Ok(CreateDeploymentRequest {
			name: self.name.resolve_value()?,
			registry,
			image_tag,
			deploy_on_create: false,
			runner,
			machine_type: Uuid::parse_str(&self.machine_type.resolve_value()?.to_string()).unwrap(),
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
				volumes: Default::default(),
			},
		})
	}
}
