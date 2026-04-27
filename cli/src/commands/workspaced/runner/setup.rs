use std::{
	collections::BTreeSet,
	iter,
	net::{IpAddr, SocketAddr},
	str::FromStr,
	time::{Duration, Instant},
};

use clap::Args as ClapArgs;
use common::prelude::{DatabaseConfig, RunnerMode, RunnerSettings, RunningEnvironment};
use docker::prelude::DockerSettings;
use inquire::{CustomType, MultiSelect, Select, Text};
use models::{
	ApiErrorResponseBody,
	api::{user::*, workspace::runner::*},
	utils::{BearerToken, False},
};
use rand::RngExt;

use crate::prelude::*;

/// Args for `patr runner setup`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Force the setup even if the CLI is already configured
	#[arg(short = 'f', long = "force")]
	pub force: bool,
}

/// First-time configuration for a runner on this host — writes the runner
/// config file used by `patr runner run` and `patr runner service install`.
pub async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let config_path = crate::utils::runner_config_path();

	if config_path.exists() && !args.force {
		let message = concat!(
			"A runner configuration already exists. ",
			"To override it, use the `--force` flag."
		);
		return CommandOutput::builder()
			.text(message)
			.json(
				ApiErrorResponseBody {
					success: False,
					error: ErrorType::ResourceAlreadyExists,
					message: message.to_string(),
				}
				.to_json_value(),
			)
			.build()
			.into_result();
	}

	// Prompt for runner mode
	const MANAGED_OPTION: &str = "Managed (connect to Patr cloud)";
	const SELF_HOSTED_OPTION: &str = "[Beta] Self-hosted (standalone)";

	let mode_options = vec![MANAGED_OPTION, SELF_HOSTED_OPTION];
	let mode_selection = Select::new("Select runner mode:", mode_options)
		.prompt()
		.expect_tty("Failed to read runner mode");

	let is_managed = mode_selection == MANAGED_OPTION;

	let mode = if is_managed {
		let AuthState::LoggedIn {
			token,
			current_workspace,
		} = state.auth
		else {
			return Err(AppError::NotLoggedIn);
		};

		// Pick the workspace: prefer --workspace flag, then a Select pre-positioned
		// on the CLI's current_workspace. One-workspace accounts skip the prompt.
		let workspaces = make_request(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.workspaces;

		let workspace_id = if let Some(arg) = global_args.workspace.as_deref() {
			workspaces
				.into_iter()
				.find(|w| w.id.to_string() == arg || w.name == arg)
				.map(|w| w.id)
				.ok_or_else(|| {
					AppError::ParseError(format!("No workspace found with ID or name: `{arg}`"))
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
			let selected = Select::new("Select the workspace to add this runner to:", names)
				.with_starting_cursor(starting_cursor)
				.prompt()
				.expect_tty("Failed to read workspace selection");
			workspaces
				.into_iter()
				.find(|w| w.name == selected)
				.expect("selected name came from the workspace list")
				.id
		};

		// Drives the consent-link handshake: collects machine info, opens the
		// browser, polls until the user approves on the web UI.
		let version = env!("CARGO_PKG_VERSION").parse().map_err(|e| {
			AppError::ParseError(format!("Failed to parse CLI version as semver: {e}"))
		})?;
		let os = std::env::consts::OS.to_string();
		let arch = std::env::consts::ARCH.to_string();
		let hostname = hostname::get()
			.map_err(|e| AppError::ParseError(format!("Failed to read hostname: {e}")))?
			.to_string_lossy()
			.to_string();
		let private_ip = if_addrs::get_if_addrs()
			.unwrap_or_default()
			.into_iter()
			.map(|iface| iface.ip())
			.find(|ip| !ip.is_loopback())
			.unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

		let create = make_request(
			ApiRequest::<CreateRunnerLinkRequest>::builder()
				.path(CreateRunnerLinkPath { workspace_id })
				.headers(CreateRunnerLinkRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.body(CreateRunnerLinkRequest {
					version,
					os,
					arch,
					hostname,
					private_ip,
				})
				.build(),
		)
		.await?
		.body;

		println!("Opening approval request in your browser...",);
		println!();
		println!("If your browser did not open, visit this URL to approve the runner:");
		println!("{}", create.verification_uri);
		println!();
		println!("If prompted for a code, enter: {}", create.user_code);
		println!();
		// Best-effort browser launch. Silently no-ops on headless machines —
		// the URL is already on the terminal so the user can open it elsewhere.
		_ = open::that(&create.verification_uri_complete);
		println!("Waiting for approval...");

		let interval = Duration::from_secs(create.interval);
		let deadline = Instant::now() + Duration::from_secs(create.expires_in);
		loop {
			tokio::time::sleep(interval).await;
			if Instant::now() >= deadline {
				return Err(AppError::ParseError(
					"Approval timed out. Re-run `patr runner setup` to try again.".to_string(),
				));
			}

			let response = match make_request(
				ApiRequest::<VerifyRunnerLinkRequest>::builder()
					.path(VerifyRunnerLinkPath { workspace_id })
					.headers(VerifyRunnerLinkRequestHeaders {
						authorization: token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.body(VerifyRunnerLinkRequest {
						user_code: create.user_code.clone(),
						device_code: create.device_code.clone(),
					})
					.build(),
			)
			.await
			{
				Ok(resp) => resp.body,
				// Server has no record of this link — could be expired (TTL race
				// against our local deadline check), or already claimed in another
				// session. Either way, the user needs to start over.
				Err(err) if matches!(err.body.error, ErrorType::ResourceDoesNotExist) => {
					return Err(AppError::ParseError(
						"Approval link is no longer valid. Re-run `patr runner setup` to start over."
							.to_string(),
					));
				}
				Err(err) => return Err(err.into()),
			};

			match response.result {
				VerifyRunnerLinkResult::Pending => continue,
				VerifyRunnerLinkResult::Approved {
					runner_id,
					workspace_id,
					token,
				} => {
					println!("✓ Runner approved");
					break RunnerMode::Managed {
						workspace_id,
						runner_id,
						api_token: BearerToken::from_str(&token).map_err(|e| {
							AppError::ParseError(format!("Server returned malformed token: {e}"))
						})?,
						user_agent: constants::USER_AGENT,
					};
				}
			}
		}
	} else {
		let mut rng = rand::rng();
		let pepper_bytes: [u8; 32] = rng.random();
		let jwt_bytes: [u8; 32] = rng.random();
		let password_pepper = pepper_bytes
			.iter()
			.map(|b| format!("{b:02x}"))
			.collect::<String>();
		let jwt_secret = jwt_bytes
			.iter()
			.map(|b| format!("{b:02x}"))
			.collect::<String>();

		RunnerMode::SelfHosted {
			password_pepper,
			jwt_secret,
		}
	};

	// Prompt for Docker settings
	let bind_address_str = Text::new("Bind address:")
		.with_default("127.0.0.1:4000")
		.with_help_message("The address the runner's API server will listen on")
		.prompt()
		.expect_tty("Failed to read bind address");
	let bind_address: SocketAddr = bind_address_str
		.parse()
		.map_err(|e| AppError::ParseError(format!("Invalid bind address: {e}")))?;

	let docker_swarm_listen_addr = Text::new("Docker Swarm listen address:")
		.with_default("127.0.0.1:2377")
		.with_help_message("The address Docker Swarm will listen on for cluster management")
		.prompt()
		.expect_tty("Failed to read swarm address");

	let ingress_http_listen_port = CustomType::<u16>::new("HTTP ingress port:")
		.with_default(80)
		.with_help_message("The port the ingress will use for HTTP traffic")
		.prompt()
		.expect_tty("Failed to read HTTP port");

	let ingress_https_listen_port = CustomType::<u16>::new("HTTPS ingress port:")
		.with_default(443)
		.with_help_message("The port the ingress will use for HTTPS traffic")
		.prompt()
		.expect_tty("Failed to read HTTPS port");

	// Exposure type — managed mode can use Private (tunnel), self-hosted cannot
	let exposure_options = if is_managed {
		vec!["Private (tunnel via Patr)", "Public IP", "Public DNS"]
	} else {
		vec!["Public IP", "Public DNS"]
	};
	let exposure_selection = Select::new("How will deployments be reachable?", exposure_options)
		.with_help_message("How your deployments will be reachable from the internet")
		.prompt()
		.expect_tty("Failed to read exposure type");

	let runner_exposure_type = if exposure_selection.starts_with("Private") {
		RunnerExposureType::Private
	} else if exposure_selection == "Public IP" {
		// Detect local IPs from network interfaces to offer as choices
		let detected_ips = if_addrs::get_if_addrs()
			.unwrap_or_default()
			.into_iter()
			.map(|iface| iface.ip())
			.filter(|ip| !ip.is_loopback())
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect::<Vec<_>>();

		const ENTER_MANUALLY: &str = "Enter more manually";

		let ip_selections = MultiSelect::new(
			"Select public IP address(es):",
			detected_ips
				.iter()
				.map(|ip| ip.to_string())
				.chain(iter::once(ENTER_MANUALLY.to_string()))
				.collect(),
		)
		.with_help_message("Detected IPs from this machine's network interfaces. Select one or more, or choose 'Enter manually'")
		.prompt()
		.expect_tty("Failed to read IP selection");

		let mut ip_addresses = ip_selections
			.iter()
			.filter(|s| *s != ENTER_MANUALLY)
			.map(|s| {
				s.parse()
					.map_err(|e| AppError::ParseError(format!("Invalid IP address: {e}")))
			})
			.collect::<Result<Vec<_>, _>>()?;

		if ip_selections.iter().any(|s| s == ENTER_MANUALLY) {
			let ip_input = Text::new("Additional public IP address(es):")
				.with_help_message("Enter additional public IP address(es), comma-separated")
				.prompt()
				.expect_tty("Failed to read IP addresses");

			let manual_ips: Vec<IpAddr> = ip_input
				.split(',')
				.map(|s| {
					s.trim()
						.parse()
						.map_err(|e| AppError::ParseError(format!("Invalid IP address: {e}")))
				})
				.collect::<Result<Vec<_>, _>>()?;
			ip_addresses.extend(manual_ips);
		}

		if ip_addresses.is_empty() {
			return Err(AppError::ParseError(
				"At least one IP address is required".to_string(),
			));
		}

		RunnerExposureType::PublicIP { ip_addresses }
	} else {
		let dns_name = Text::new("Public DNS name:")
			.with_help_message(
				"The public DNS name that points to this machine (e.g. runner.example.com)",
			)
			.prompt()
			.expect_tty("Failed to read DNS name");
		RunnerExposureType::PublicDNS { dns_name }
	};

	let config = RunnerSettings {
		mode,
		environment: if cfg!(debug_assertions) {
			RunningEnvironment::Development
		} else {
			RunningEnvironment::Production
		},
		database: DatabaseConfig {
			file: config_path
				.with_extension("db")
				.to_string_lossy()
				.to_string(),
			connection_limit: 10,
		},
		bind_address,
		data: DockerSettings {
			docker_swarm_listen_addr,
			ingress_http_listen_port,
			ingress_https_listen_port,
			runner_exposure_type,
			enable_ipv6: true,
		},
	};

	let json = serde_json::to_string_pretty(&config)
		.map_err(|e| AppError::ParseError(format!("Failed to serialize config: {e}")))?;

	if let Some(parent) = config_path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|e| AppError::ParseError(format!("Failed to create config directory: {e}")))?;
	}

	std::fs::write(&config_path, json)
		.map_err(|e| AppError::ParseError(format!("Failed to write config file: {e}")))?;

	CommandOutput::builder()
		.text(format!(
			"Runner configured successfully! Config written to {}.\nRun with: patr runner run docker",
			config_path.display()
		))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
