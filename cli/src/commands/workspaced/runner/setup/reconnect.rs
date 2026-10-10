use std::{
	fs::{OpenOptions, Permissions},
	io::{IsTerminal, Write},
	os::unix::fs::{OpenOptionsExt, PermissionsExt},
	path::PathBuf,
	str::FromStr,
};

use clap::Args as ClapArgs;
use common::prelude::{RunnerMode, RunnerSettings, RunningEnvironment};
use docker::prelude::DockerSettings;
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use models::api::{user::*, workspace::runner::*};

use super::{SetupOutput, prompt_settings};
use crate::prelude::*;

/// Args for `patr runner setup reconnect`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The ID of the runner. Asks if not given.
	#[arg(long = "runner-id")]
	pub runner_id: Option<Uuid>,
	/// The runner's token (`patr_sa_…`). Without it, a logged-in CLI
	/// regenerates the token, and a logged-out one asks for it.
	#[arg(
		long = "runner-token",
		env = "PATR_RUNNER_TOKEN",
		hide_env_values = true
	)]
	pub runner_token: Option<String>,
	/// Regenerate the runner's token without asking for confirmation
	#[arg(short = 'y', long = "yes")]
	pub yes: bool,
}

/// Sets this host up to run an existing runner. Reconnecting the runner this
/// host already runs keeps its settings and only swaps the token.
pub(super) async fn execute(
	args: Args,
	config_path: PathBuf,
	force: bool,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let (user_token, current_workspace) = match state.auth {
		AuthState::LoggedIn {
			token,
			current_workspace,
		} => (Some(token), current_workspace),
		AuthState::LoggedOut {} => (None, None),
	};

	// A service account can't list workspaces, so without a login the
	// workspace has to be given by ID.
	let workspace_id = if let Some(user_token) = &user_token {
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

		if let Some(workspace) = global_args.workspace {
			workspaces
				.into_iter()
				.find(|w| w.id.to_string() == workspace || w.name == workspace)
				.map(|w| w.id)
				.ok_or_else(|| {
					AppError::ParseError(format!(
						"No workspace found with ID or name: `{workspace}`"
					))
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
		}
	} else {
		let workspace = global_args.workspace.unwrap_or_else(|| {
			Text::new("Workspace ID:")
				.prompt()
				.expect_tty("Failed to read workspace ID")
		});
		workspace.parse::<Uuid>().map_err(|_| {
			AppError::ParseError(format!(
				"`{workspace}` isn't a workspace ID. Without logging in, the workspace must be given by ID."
			))
		})?
	};

	let runner_id = match (args.runner_id, &user_token) {
		(Some(runner_id), _) => runner_id,
		(None, Some(user_token)) => {
			let runners = make_request(
				ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
					.path(ListRunnersForWorkspacePath { workspace_id })
					.headers(ListRunnersForWorkspaceRequestHeaders {
						authorization: user_token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.build(),
			)
			.await?
			.body
			.runners;

			if runners.is_empty() {
				return Err(AppError::RunnerError(
					"This workspace has no runners. Run `patr runner setup new` to create one."
						.to_string(),
				));
			}

			let names = runners
				.iter()
				.map(|r| r.name.clone())
				.collect::<Vec<String>>();
			let selected = Select::new("Select the runner to reconnect:", names)
				.prompt()
				.expect_tty("Failed to read runner selection");
			runners
				.into_iter()
				.find(|r| r.name == selected)
				.expect("selected name came from the runner list")
				.id
		}
		(None, None) => {
			let runner = Text::new("Runner ID:")
				.prompt()
				.expect_tty("Failed to read runner ID");
			runner
				.parse::<Uuid>()
				.map_err(|_| AppError::ParseError(format!("`{runner}` isn't a runner ID")))?
		}
	};

	// Another runner's config is replaced only with `--force`. This same
	// runner's is kept, and only gets the new token.
	let existing = std::fs::read_to_string(&config_path)
		.ok()
		.and_then(|config| serde_json::from_str::<RunnerSettings<DockerSettings>>(&config).ok())
		.filter(|config| {
			matches!(
				config.mode,
				RunnerMode::Managed {
					workspace_id: existing_workspace_id,
					runner_id: existing_runner_id,
					..
				} if existing_workspace_id == workspace_id && existing_runner_id == runner_id
			)
		});
	if existing.is_none() && config_path.exists() && !force {
		return Err(AppError::RunnerError(format!(
			"A config for another runner already exists at {}. Replacing it with `--force` can \
			 make this runner remove the other runner's deployments from this machine, volumes \
			 included. Move or delete {} first if you mean to switch runners.",
			config_path.display(),
			config_path.with_extension("db").display()
		)));
	}

	// Without a token, a logged-in CLI regenerates it below, and a logged-out
	// one asks for it.
	let runner_token = match (args.runner_token, &user_token) {
		(None, Some(_)) => None,
		(runner_token, _) => {
			let runner_token = runner_token.unwrap_or_else(|| {
				Password::new("Runner token:")
					.with_display_mode(PasswordDisplayMode::Masked)
					.with_help_message(
						"Shown once when the runner is created or its token is regenerated",
					)
					.without_confirmation()
					.prompt()
					.expect_tty("Failed to read runner token")
			});
			if !runner_token.starts_with("patr_sa_") {
				return Err(AppError::ParseError(
					"That isn't a runner token. A runner's token starts with `patr_sa_`."
						.to_string(),
				));
			}
			Some(BearerToken::from_str(&runner_token)?)
		}
	};

	// With a runner token, this also checks the token belongs to the runner.
	let name = make_request(
		ApiRequest::<GetRunnerInfoRequest>::builder()
			.path(GetRunnerInfoPath {
				workspace_id,
				runner_id,
			})
			.headers(GetRunnerInfoRequestHeaders {
				authorization: runner_token
					.clone()
					.or_else(|| user_token.clone())
					.expect("a runner token, or a login to regenerate one"),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.runner
	.data
	.name;

	if runner_token.is_none() {
		eprintln!(
			"This regenerates the token for runner `{name}`. The machine running it now \
			 disconnects within 30 seconds and stays offline until it's given the new token. \
			 Its deployments keep running and nothing is redeployed, but if it uses a private \
			 tunnel, their URLs are down until it reconnects."
		);
		if !args.yes {
			if !std::io::stdin().is_terminal() {
				return Err(AppError::RunnerError(
					"Running in non-TTY mode. Pass `-y` to confirm.".to_string(),
				));
			}
			let confirmed = Confirm::new("Regenerate the runner's token?")
				.with_default(false)
				.prompt()
				.expect_tty("Failed to read confirmation");
			if !confirmed {
				return Err(AppError::RunnerError("Aborted.".to_string()));
			}
		}
	}

	let token_only = existing.is_some();
	let (database, bind_address, data) = match existing {
		Some(existing) => (existing.database, existing.bind_address, existing.data),
		None => prompt_settings(&config_path)?,
	};

	// Regenerated only after every question, so an aborted prompt doesn't
	// disconnect the runner.
	let api_token = match runner_token {
		Some(runner_token) => runner_token,
		None => {
			let token = make_request(
				ApiRequest::<RegenerateRunnerTokenRequest>::builder()
					.path(RegenerateRunnerTokenPath {
						workspace_id,
						runner_id,
					})
					.headers(RegenerateRunnerTokenRequestHeaders {
						authorization: user_token.expect("only regenerated when logged in"),
						user_agent: constants::USER_AGENT,
					})
					.build(),
			)
			.await?
			.body
			.token;
			BearerToken::from_str(&token)?
		}
	};

	let config = RunnerSettings {
		mode: RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
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
		.text(
			if token_only {
				format!(
					"Updated the token for runner `{name}` in {}.\n\
				 If it runs as a service, restart it to use the new token:\n\
				 sudo systemctl restart patr-docker-runner",
					config_path.display()
				)
			} else {
				format!(
					"Runner `{name}` is set up. Config written to {}.\n\
				 Run it with `patr runner run`, or install it as a service with \
				 `patr runner service install`.",
					config_path.display()
				)
			},
		)
		.json(
			SetupOutput {
				config_path: config_path.display().to_string(),
				workspace_id,
				runner_id,
				name,
			}
			.to_json_value(),
		)
		.build()
		.into_result()
}
