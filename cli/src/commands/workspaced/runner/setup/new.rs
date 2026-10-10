use std::{
	fs::{OpenOptions, Permissions},
	io::Write,
	os::unix::fs::{OpenOptionsExt, PermissionsExt},
	path::PathBuf,
	str::FromStr,
};

use clap::Args as ClapArgs;
use common::prelude::{RunnerMode, RunnerSettings, RunningEnvironment};
use inquire::{Select, Text};
use models::api::{user::*, workspace::runner::*};

use super::{SetupOutput, prompt_settings};
use crate::prelude::*;

/// Args for `patr runner setup new`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name of the new runner. Asks if not given.
	#[arg(short = 'n', long = "name")]
	pub name: Option<String>,
}

/// Creates a runner in the workspace and sets this host up to run it.
pub(super) async fn execute(
	args: Args,
	config_path: PathBuf,
	force: bool,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn {
		token: user_token,
		current_workspace,
	} = state.auth
	else {
		return Err(AppError::NotLoggedIn);
	};
	if config_path.exists() && !force {
		return Err(AppError::RunnerError(format!(
			"A runner config already exists at {}. Replacing it with `--force` can make the new \
			 runner remove the old runner's deployments from this machine, volumes included. Move \
			 or delete {} first if you mean to switch runners.",
			config_path.display(),
			config_path.with_extension("db").display()
		)));
	}

	let workspaces = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: user_token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.workspaces;

	let workspace_id = if let Some(workspace) = global_args.workspace {
		workspaces
			.into_iter()
			.find(|w| w.id.to_string() == workspace || w.name == workspace)
			.map(|w| w.id)
			.ok_or_else(|| {
				AppError::ParseError(format!("No workspace found with ID or name: `{workspace}`"))
			})?
	} else if workspaces.len() == 1 {
		workspaces.into_iter().next().unwrap().id
	} else {
		let names = workspaces
			.iter()
			.map(|w| w.name.clone())
			.collect::<Vec<String>>();
		let starting_cursor = current_workspace
			.and_then(|id| workspaces.iter().position(|w| w.id == id))
			.unwrap_or(0);
		let selected = Select::new("Select the workspace the runner belongs to:", names)
			.with_starting_cursor(starting_cursor)
			.prompt()
			.expect_tty("Failed to read workspace selection");
		workspaces
			.into_iter()
			.find(|w| w.name == selected)
			.expect("selected name came from the workspace list")
			.id
	};

	let name = args.name.unwrap_or_else(|| {
		Text::new("Runner name:")
			.prompt()
			.expect_tty("Failed to read runner name")
	});

	// Ask everything before creating the runner, so an aborted prompt doesn't
	// leave an unused runner behind.
	let (database, bind_address, data) = prompt_settings(&config_path)?;

	let CreateRunnerResponse { id, token } = make_request(
		ApiRequest::<CreateRunnerRequest>::builder()
			.path(CreateRunnerPath { workspace_id })
			.headers(CreateRunnerRequestHeaders {
				authorization: user_token,
				user_agent: constants::USER_AGENT,
			})
			.body(CreateRunnerRequest { name: name.clone() })
			.build(),
	)
	.await?
	.body;

	let config = RunnerSettings {
		mode: RunnerMode::Managed {
			workspace_id,
			runner_id: id.id,
			api_token: BearerToken::from_str(&token)?,
			user_agent: constants::USER_AGENT,
		},
		environment: if cfg!(debug_assertions) {
			RunningEnvironment::Development
		} else {
			RunningEnvironment::Production
		},
		database,
		bind_address,
		data,
	};

	let json = serde_json::to_string_pretty(&config)
		.map_err(|e| AppError::ParseError(format!("Failed to serialize config: {e}")))?;

	if let Some(parent) = config_path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|e| AppError::ParseError(format!("Failed to create config directory: {e}")))?;
	}

	// The config holds the runner's token, so only its owner can read it.
	OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600)
		.open(&config_path)
		.and_then(|mut file| {
			file.set_permissions(Permissions::from_mode(0o600))?;
			file.write_all(json.as_bytes())
		})
		.map_err(|e| AppError::ParseError(format!("Failed to write config file: {e}")))?;

	CommandOutput::builder()
		.text(format!(
			"Runner `{name}` is set up. Config written to {}.\n\
			 Run it with `patr runner run`, or install it as a service with \
			 `patr runner service install`.",
			config_path.display()
		))
		.json(
			SetupOutput {
				config_path: config_path.display().to_string(),
				workspace_id,
				runner_id: id.id,
				name,
			}
			.to_json_value(),
		)
		.build()
		.into_result()
}
