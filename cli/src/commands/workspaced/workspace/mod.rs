
mod create;
mod list;
mod rename;
mod switch;

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommands {
	#[subcommand(rename = "workspace")]
	WorkspaceAction(WorkspaceActionCommands),
	#[subcommand]
	Context(ContextCommands),
	#[subcommand(rename = "workspaces")]
	ListWorkspaces,
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceActionCommands {
	Create(CreateArgs),
	Switch(SwitchArgs),
	List(ListArgs),
	Rename(RenameArgs),
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ContextCommands {
	Switch {
		name: String,
	}
}

#[async_trait::async_trait]
impl CommandExecutor for WorkspaceCommands {
    async fn execute(
        self,
        global_args: &GlobalArgs,
        writer: impl Write + Send,
    ) -> anyhow::Result<()> {
        match self {
            Self::WorkspaceAction(commands) => commands.execute(global_args, writer).await,
            Self::Context(ContextCommands::Switch {
				name
			}) => WorkspaceActionCommands::Switch {
				name
			}.execute(global_args, writer).await,
            Self::ListWorkspaces => WorkspaceActionCommands::List.execute(global_args, writer).await,
        }
    }
}

#[async_trait::async_trait]
impl CommandExecutor for WorkspaceActionCommands {
	async fn execute(
		self,
		global_args: &GlobalArgs,
		writer: impl Write + Send,
	) -> anyhow::Result<()> {
		match self {
			Self::Create(args) => create::execute(global_args, args, writer).await,
			Self::Switch(args) => switch::execute(global_args, args, writer).await,
			Self::List(args) => list::execute(global_args, args, writer).await,
			Self::Rename(args) => rename::execute(global_args, args, writer).await,
		}
	}
}
