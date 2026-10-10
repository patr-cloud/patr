//! `patr runner setup`.

use std::{iter, os::unix::fs::PermissionsExt, path::Path, process::Stdio};

use cli::prelude::*;
use common::prelude::{DatabaseConfig, RunnerMode, RunnerSettings, RunningEnvironment};
use docker::prelude::DockerSettings;
use models::api::{
	user::*,
	workspace::{Workspace, runner::*},
};
use tempfile::TempDir;
use wiremock::{
	Mock,
	MockServer,
	matchers::{header, method, path},
};

use crate::{apply::requests, setup};

/// The token a logged-in [`setup::state`] sends.
const USER_TOKEN: &str = "patrv1.test-token";
/// The token in a config written before the test runs.
const OLD_TOKEN: &str = "patr_sa_old-token";
/// The token the stub API hands out, and the one tests pass on the command
/// line.
const NEW_TOKEN: &str = "patr_sa_new-token";

struct Ids {
	workspace: Uuid,
	runner: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000021").unwrap(),
		}
	}
}

/// Run `patr <argv>` against the stub API.
async fn run(state: AppState, argv: &[&str]) -> Result<CommandOutput, AppError> {
	use clap::Parser;

	let AppArgs { args, command } =
		AppArgs::try_parse_from(iter::once("patr").chain(argv.iter().copied()))
			.expect("failed to parse args");

	cli::commands::execute(command, args, state).await
}

/// Mount `GET /user/workspaces` returning the test workspace.
async fn mount_workspaces(server: &MockServer, ids: &Ids) {
	Mock::given(method("GET"))
		.and(path("/user/workspaces"))
		.respond_with(setup::success(ListUserWorkspacesResponse {
			workspaces: vec![WithId::new(
				ids.workspace,
				Workspace {
					name: "test-workspace".to_string(),
					super_admin_id: Uuid::nil(),
				},
			)],
		}))
		.mount(server)
		.await;
}

/// Mount `GET /workspace/{id}/runner/{runner_id}`, answering only requests
/// that carry `token`.
async fn mount_runner_info(server: &MockServer, ids: &Ids, token: &str) {
	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{}/runner/{}",
			ids.workspace, ids.runner
		)))
		.and(header("authorization", format!("Bearer {token}").as_str()))
		.respond_with(setup::success(GetRunnerInfoResponse {
			runner: WithId::new(
				ids.runner,
				Runner {
					name: "test-runner".to_string(),
					connected: true,
					last_seen: None,
					version: "0.1.0".parse().expect("valid semver"),
				},
			),
		}))
		.mount(server)
		.await;
}

/// Write a config for `runner_id` holding [`OLD_TOKEN`], with a bind address
/// setup wouldn't default to, so a rewrite that loses it is visible.
fn write_config(config_path: &Path, ids: &Ids, runner_id: Uuid) {
	let config = RunnerSettings {
		mode: RunnerMode::Managed {
			workspace_id: ids.workspace,
			runner_id,
			api_token: OLD_TOKEN.parse().unwrap(),
			user_agent: constants::USER_AGENT,
		},
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file: config_path
				.with_extension("db")
				.to_string_lossy()
				.to_string(),
			connection_limit: 10,
		},
		bind_address: "127.0.0.1:4100".parse().unwrap(),
		data: DockerSettings {
			docker_swarm_listen_addr: "127.0.0.1:2377".to_string(),
			ingress_http_listen_port: 8080,
			ingress_https_listen_port: 8443,
			runner_exposure_type: RunnerExposureType::Private,
			enable_ipv6: false,
		},
	};

	std::fs::write(config_path, serde_json::to_string(&config).unwrap())
		.expect("failed to write the runner config");
}

/// The runner config at `config_path`.
fn read_config(config_path: &Path) -> RunnerSettings<DockerSettings> {
	serde_json::from_str(
		&std::fs::read_to_string(config_path).expect("failed to read the runner config"),
	)
	.expect("failed to parse the runner config")
}

/// The token the runner config at `config_path` holds.
fn config_token(config_path: &Path) -> String {
	match read_config(config_path).mode {
		RunnerMode::Managed { api_token, .. } => api_token.0.token().to_string(),
		RunnerMode::SelfHosted { .. } => panic!("expected a managed runner config"),
	}
}

/// A temp dir and the runner config path inside it. The dir must outlive the
/// path.
fn config_dir() -> (TempDir, String) {
	let dir = tempfile::tempdir().expect("failed to create a temp dir");
	let config_path = dir.path().join("runner.docker.json").display().to_string();
	(dir, config_path)
}

/// A pasted runner token is checked against the runner using that token, and
/// reconnecting the runner this host already runs changes only the token.
#[tokio::test]
async fn reconnect_with_token_updates_only_the_token() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (_dir, config_path) = config_dir();

	write_config(Path::new(&config_path), &ids, ids.runner);
	mount_runner_info(server, &ids, NEW_TOKEN).await;

	let output = run(
		AppState::default(),
		&[
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"reconnect",
			"--runner-id",
			&ids.runner.to_string(),
			"--runner-token",
			NEW_TOKEN,
			"-c",
			&config_path,
		],
	)
	.await
	.expect("setup failed");

	let config_path = Path::new(&config_path);
	let config = read_config(config_path);
	assert_eq!(config_token(config_path), NEW_TOKEN);
	assert_eq!(config.bind_address, "127.0.0.1:4100".parse().unwrap());
	assert_eq!(config.data.ingress_http_listen_port, 8080);
	assert_eq!(
		std::fs::metadata(config_path).unwrap().permissions().mode() & 0o777,
		0o600
	);

	assert_eq!(
		output.json["runnerId"].as_str(),
		Some(ids.runner.to_string().as_str())
	);
	assert_eq!(output.json["name"].as_str(), Some("test-runner"));
	assert!(
		!output.json.to_string().contains(NEW_TOKEN),
		"the JSON output leaks the runner's token"
	);
}

/// A user's API token isn't a runner token, and is refused before anything is
/// sent or written.
#[tokio::test]
async fn reconnect_rejects_a_user_token() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (_dir, config_path) = config_dir();

	let result = run(
		AppState::default(),
		&[
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"reconnect",
			"--runner-id",
			&ids.runner.to_string(),
			"--runner-token",
			"patr_at_user-token",
			"-c",
			&config_path,
		],
	)
	.await;

	assert!(result.is_err(), "a `patr_at_` token was accepted");
	assert!(requests(server).await.is_empty());
	assert!(!Path::new(&config_path).exists());
}

/// A config for another runner isn't replaced without `--force`.
#[tokio::test]
async fn reconnect_refuses_another_runners_config() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (_dir, config_path) = config_dir();

	write_config(
		Path::new(&config_path),
		&ids,
		Uuid::parse_str("00000000000000000000000000000022").unwrap(),
	);

	let result = run(
		AppState::default(),
		&[
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"reconnect",
			"--runner-id",
			&ids.runner.to_string(),
			"--runner-token",
			NEW_TOKEN,
			"-c",
			&config_path,
		],
	)
	.await;

	assert!(result.is_err(), "another runner's config was replaced");
	assert!(requests(server).await.is_empty());
	assert_eq!(config_token(Path::new(&config_path)), OLD_TOKEN);
}

/// Logged in and without a token, `-y` regenerates the runner's token without
/// asking.
#[tokio::test]
async fn reconnect_regenerates_the_token_with_yes() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (_dir, config_path) = config_dir();

	write_config(Path::new(&config_path), &ids, ids.runner);
	mount_workspaces(server, &ids).await;
	mount_runner_info(server, &ids, USER_TOKEN).await;
	Mock::given(method("POST"))
		.and(path(format!(
			"/workspace/{}/runner/{}/token",
			ids.workspace, ids.runner
		)))
		.and(header(
			"authorization",
			format!("Bearer {USER_TOKEN}").as_str(),
		))
		.respond_with(setup::success(RegenerateRunnerTokenResponse {
			token: NEW_TOKEN.to_string(),
		}))
		.expect(1)
		.mount(server)
		.await;

	run(
		setup::state(ids.workspace),
		&[
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"reconnect",
			"--runner-id",
			&ids.runner.to_string(),
			"-y",
			"-c",
			&config_path,
		],
	)
	.await
	.expect("setup failed");

	assert_eq!(config_token(Path::new(&config_path)), NEW_TOKEN);
	server.verify().await;
}

/// Without a TTY or `-y`, the token isn't regenerated. This runs the binary
/// with a null stdin, since the test's own stdin is a terminal when the suite
/// is run by hand.
#[tokio::test]
async fn reconnect_regenerate_needs_yes_without_a_tty() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (dir, config_path) = config_dir();

	write_config(Path::new(&config_path), &ids, ids.runner);
	mount_workspaces(server, &ids).await;
	mount_runner_info(server, &ids, USER_TOKEN).await;

	let state_path = dir.path().join("cli.json");
	std::fs::write(
		&state_path,
		serde_json::to_string(&setup::state(ids.workspace)).unwrap(),
	)
	.expect("failed to write the CLI state");

	let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_patr"))
		.args([
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"reconnect",
			"--runner-id",
			&ids.runner.to_string(),
			"-c",
			&config_path,
		])
		.env("HOME", dir.path())
		.env("CONFIG_PATH", &state_path)
		.env_remove("XDG_DATA_HOME")
		.env_remove("PATR_RUNNER_TOKEN")
		.stdin(Stdio::null())
		.output()
		.await
		.expect("failed to run patr");

	assert!(
		!output.status.success(),
		"setup regenerated the token without `-y`"
	);
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("Pass `-y` to confirm"),
		"setup failed for another reason: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(
		requests(server)
			.await
			.iter()
			.map(|req| format!("{} {}", req.method, req.url.path()))
			.collect::<Vec<_>>(),
		[
			"GET /user/workspaces".to_string(),
			format!("GET /workspace/{}/runner/{}", ids.workspace, ids.runner),
		]
	);
	assert_eq!(config_token(Path::new(&config_path)), OLD_TOKEN);
}

/// Creating a runner needs a login, and fails before anything is sent.
#[tokio::test]
async fn new_needs_a_login() {
	let ids = Ids::new();
	let server = setup::reset().await;
	let (_dir, config_path) = config_dir();

	let result = run(
		AppState::default(),
		&[
			"-w",
			&ids.workspace.to_string(),
			"runner",
			"setup",
			"new",
			"--name",
			"test-runner",
			"-c",
			&config_path,
		],
	)
	.await;

	assert!(matches!(result, Err(AppError::NotLoggedIn)));
	assert!(requests(server).await.is_empty());
	assert!(!Path::new(&config_path).exists());
}
