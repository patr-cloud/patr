//! Shared harness for the CLI integration tests.

use std::{io::Write, net::TcpListener};

use cli::prelude::*;
use models::{ApiSuccessResponseBody, utils::BearerToken};
use serde::Serialize;
use tempfile::NamedTempFile;
use tokio::sync::OnceCell;
use wiremock::{MockServer, ResponseTemplate};

/// The stub API every test talks to.
///
/// `constants::API_BASE_URL` is a compile-time constant, so the port is fixed
/// and there can only ever be one server for the whole test binary. Tests must
/// therefore run single-threaded and [`reset`] between cases.
pub async fn server() -> &'static MockServer {
	static SERVER: OnceCell<MockServer> = OnceCell::const_new();

	SERVER
		.get_or_init(|| async {
			// Without the override the CLI is built against a real API base URL
			// — in debug that's localhost:3000, which the suite would happily
			// bind and then "pass" against, while also fighting a locally
			// running API for the port.
			let address = option_env!("PATR_TEST_API_BASE_URL")
				.unwrap_or_else(|| {
					panic!(
						"the tests were built without PATR_TEST_API_BASE_URL, so the CLI points at \
						 `{}`. Run them via `just cli::test`.",
						constants::API_BASE_URL
					)
				})
				.strip_prefix("http://")
				.expect("PATR_TEST_API_BASE_URL must be an http:// address");

			let listener = TcpListener::bind(address)
				.unwrap_or_else(|err| panic!("failed to bind the stub API on `{address}`: {err}"));

			MockServer::builder().listener(listener).start().await
		})
		.await
}

/// Drop every mock and recorded request from the previous test.
pub async fn reset() -> &'static MockServer {
	let server = server().await;
	server.reset().await;
	server
}

/// A logged-in state pointing at `workspace_id`.
pub fn state(workspace_id: Uuid) -> AppState {
	AppState {
		target_channel: Channel::Alpha,
		auth: AuthState::LoggedIn {
			token: "patrv1.test-token".parse::<BearerToken>().unwrap(),
			current_workspace: Some(workspace_id),
		},
	}
}

/// A `200 OK` carrying the standard success envelope.
///
/// Built from the real response type rather than hand-written JSON so the
/// shape can't drift from what the CLI deserializes.
pub fn success(body: impl Serialize) -> ResponseTemplate {
	ResponseTemplate::new(200).set_body_json(ApiSuccessResponseBody {
		success: models::utils::True,
		response: body,
	})
}

/// A `200 OK` for a paginated list endpoint, including the `X-Total-Count`
/// header the CLI's response-header parsing requires.
pub fn success_list(body: impl Serialize, total_count: usize) -> ResponseTemplate {
	success(body).insert_header("x-total-count", total_count.to_string().as_str())
}

/// Write an IaaC config file to a temp file and hand back the handle.
///
/// The handle must stay alive for as long as the path is used — dropping it
/// deletes the file.
pub fn config_file(contents: &str) -> NamedTempFile {
	let mut file = NamedTempFile::with_suffix(".yml").expect("failed to create temp config file");
	file.write_all(contents.as_bytes())
		.expect("failed to write temp config file");
	file.flush().expect("failed to flush temp config file");
	file
}

/// Run `patr apply` against the given config file contents.
pub async fn apply(state: AppState, contents: &str, extra_args: &[&str]) -> Result<(), AppError> {
	use clap::Parser;

	let file = config_file(contents);
	let path = file.path().display().to_string();

	let mut argv = vec!["patr", "apply", "--file", path.as_str()];
	argv.extend_from_slice(extra_args);

	let AppArgs { args, command } = AppArgs::try_parse_from(argv).expect("failed to parse args");

	cli::commands::execute(command, args, state)
		.await
		.map(|_| ())
}
