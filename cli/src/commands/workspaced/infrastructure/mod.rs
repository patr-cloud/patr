use clap::Subcommand;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum InfrastructureCommands {
	// #[command(flatten)]
	// DeploymentCommands(DeploymentCommands),
	// #[command(flatten)]
	// DatabaseCommands(DatabaseCommands),
	// #[command(flatten)]
	// ContainerRegistryCommands(ContainerRegistryCommands),
	// #[command(flatten)]
	// StaticSiteCommands(StaticSiteCommands),
	// #[command(flatten)]
	// SecretCommands(SecretCommands),
}
