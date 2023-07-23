
/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspacedCommands {
    #[command(flatten)]
    WorkspaceCommands(WorkspaceCommands),
    #[command(flatten)]
    InfrastructureCommands(InfrastructureCommands),
    #[command(flatten)]
    DomainConfigurationCommands(DomainConfigurationCommands),
}

#[async_trait::async_trait]
impl CommandExecutor for WorkspaceCommands {
    async fn execute(
        self,
        global_args: &GlobalArgs,
        writer: impl Write + Send,
    ) -> anyhow::Result<()> {
        match self {
            Self::WorkspaceCommands(commands) => commands.execute(global_args, writer).await,
            Self::InfrastructureCommands(commands) => commands.execute(global_args, writer).await,
            Self::DomainConfigurationCommands(commands) => commands.execute(global_args, writer).await,
        }
    }
}