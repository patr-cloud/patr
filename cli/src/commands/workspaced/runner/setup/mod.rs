use std::{
	collections::BTreeSet,
	iter,
	net::{IpAddr, SocketAddr},
	path::{Path, PathBuf},
};

use clap::{Args as ClapArgs, Subcommand};
use common::prelude::DatabaseConfig;
use docker::prelude::DockerSettings;
use inquire::{CustomType, MultiSelect, Select, Text};
use models::api::workspace::runner::RunnerExposureType;
use serde::Serialize;

use crate::prelude::*;

/// Create a new runner and set this host up to run it
mod new;
/// Set this host up to run an existing runner
mod reconnect;

/// Args for `patr runner setup`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Whether to create a new runner or reconnect an existing one. Asks if
	/// not given.
	#[command(subcommand)]
	pub action: Option<SetupAction>,
	/// Path to write the config file to (defaults to the standard location)
	#[arg(short = 'c', long = "config", global = true)]
	pub config: Option<PathBuf>,
	/// Replace an existing config that belongs to a different runner
	#[arg(short = 'f', long = "force", global = true)]
	pub force: bool,
}

/// Whether `patr runner setup` creates a runner or reconnects to one.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SetupAction {
	/// Create a new runner in the workspace and set this host up to run it
	New(new::Args),
	/// Set this host up to run an existing runner
	Reconnect(reconnect::Args),
}

/// JSON output of `patr runner setup`. Never includes the runner's token.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupOutput {
	/// Path the config file was written to.
	config_path: String,
	/// The workspace the runner belongs to.
	workspace_id: Uuid,
	/// The runner this host is set up to run.
	runner_id: Uuid,
	/// The runner's name.
	name: String,
}

/// Sets this host up to run a runner, new or existing, by writing the config
/// file used by `patr runner run` and `patr runner service install`.
pub async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let config_path = args.config.unwrap_or_else(crate::utils::runner_config_path);

	let action = args.action.unwrap_or_else(|| {
		const NEW: &str = "New runner";
		const RECONNECT: &str = "Reconnect as an existing runner";

		let selection = Select::new("What would you like to set up?", vec![NEW, RECONNECT])
			.prompt()
			.expect_tty("Failed to read setup action");
		if selection == NEW {
			SetupAction::New(new::Args { name: None })
		} else {
			SetupAction::Reconnect(reconnect::Args {
				runner_id: None,
				runner_token: std::env::var("PATR_RUNNER_TOKEN").ok(),
				yes: false,
			})
		}
	});

	match action {
		SetupAction::New(new_args) => {
			new::execute(new_args, config_path, args.force, global_args, state).await
		}
		SetupAction::Reconnect(reconnect_args) => {
			reconnect::execute(reconnect_args, config_path, args.force, global_args, state).await
		}
	}
}

/// Asks the Docker questions for a fresh runner config.
fn prompt_settings(
	config_path: &Path,
) -> Result<(DatabaseConfig, SocketAddr, DockerSettings), AppError> {
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

	let exposure_selection = Select::new(
		"How will deployments be reachable?",
		vec!["Private (tunnel via Patr)", "Public IP", "Public DNS"],
	)
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

	Ok((
		DatabaseConfig {
			file: config_path
				.with_extension("db")
				.to_string_lossy()
				.to_string(),
			connection_limit: 10,
		},
		bind_address,
		DockerSettings {
			docker_swarm_listen_addr,
			ingress_http_listen_port,
			ingress_https_listen_port,
			runner_exposure_type,
			enable_ipv6: true,
		},
	))
}
