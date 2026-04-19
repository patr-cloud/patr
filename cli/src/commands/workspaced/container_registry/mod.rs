use clap::Subcommand;

use self::image::ImageCommand;
use crate::prelude::*;

/// Build a Dockerfile and (optionally) push it to the Patr registry
mod build;
/// Create a new container registry repository
mod create;
/// Delete a container registry repository
mod delete;
/// Image (manifest) related commands
mod image;
/// List all container registry repositories
mod list;
/// Log the local docker CLI into registry.patr.cloud
mod login;
/// Tag a local image with the Patr registry URL and push it
mod push;
/// List tags for a container registry repository
mod tags;
/// Print the full registry image URL for a repository
mod url;

/// All container registry related commands
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ContainerRegistryCommand {
	/// The commands that can be executed on container registries
	#[command(
		subcommand,
		name = "registry",
		alias = "repo",
		alias = "container-registry",
		alias = "container-repository",
		alias = "repository"
	)]
	RegistryAction(RegistryActionCommand),
	/// List all container registry repositories in the workspace
	#[command(
		name = "registries",
		alias = "repos",
		alias = "container-registries",
		alias = "container-repositories",
		alias = "repositories"
	)]
	ListRegistries,
}

/// The actions that can be performed on container registries
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum RegistryActionCommand {
	/// Create a new container registry repository
	#[command(alias = "new", alias = "add")]
	Create(create::Args),
	/// List all container registry repositories in the workspace
	#[command(alias = "ls")]
	List,
	/// List tags for a container registry repository
	Tags(tags::Args),
	/// Delete a container registry repository
	#[command(alias = "remove", alias = "rm")]
	Delete(delete::Args),
	/// Image (manifest) related commands
	#[command(subcommand)]
	Image(ImageCommand),
	/// Print the full registry image URL for a repository
	Url(url::Args),
	/// Log the local docker CLI into registry.patr.cloud using your API token
	#[command(alias = "docker-login")]
	Login,
	/// Tag a local image with the Patr registry URL and push it
	Push(push::Args),
	/// Build a Dockerfile and (optionally) push it to the Patr registry
	Build(build::Args),
}

/// Execute a container registry command
pub async fn execute(
	command: ContainerRegistryCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	use RegistryActionCommand::*;

	match command {
		ContainerRegistryCommand::RegistryAction(Create(args)) => {
			create::execute(args, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(List) |
		ContainerRegistryCommand::ListRegistries => list::execute(global_args, state).await,
		ContainerRegistryCommand::RegistryAction(Tags(args)) => {
			tags::execute(args, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(Delete(args)) => {
			delete::execute(args, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(Image(command)) => {
			image::execute(command, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(Url(args)) => {
			url::execute(args, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(Login) => login::execute(global_args, state).await,
		ContainerRegistryCommand::RegistryAction(Push(args)) => {
			push::execute(args, global_args, state).await
		}
		ContainerRegistryCommand::RegistryAction(Build(args)) => {
			build::execute(args, global_args, state).await
		}
	}
}
