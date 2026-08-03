//! Integration tests for the interactive deployment-shell websocket
//! (`GET /workspace/{id}/deployment/{id}/shell`) and its runner-facing
//! dial-back counterpart. These are the first tests in the suite to drive a
//! websocket endpoint (via axum-test's `ws` support).

use axum_test::TestWebSocket;
use futures::StreamExt;
use models::{
	api::workspace::{
		deployment::{
			StreamDeploymentShellClientMsg,
			StreamDeploymentShellPath,
			StreamDeploymentShellRequestHeaders,
			StreamDeploymentShellServerMsg,
		},
		runner::{
			StreamRunnerShellConnectionClientMsg,
			StreamRunnerShellConnectionPath,
			StreamRunnerShellConnectionRequestHeaders,
		},
	},
	rbac::{DeploymentPermission, Permission},
	utils::Uuid,
};
use rustis::commands::{ListCommands, PubSubCommands};

use crate::prelude::*;

/// Store a live shell-session record bound to `runner_id`, mirroring what the
/// CLI-facing handler writes at mint time.
async fn seed_session(
	setup: &TestSetup,
	session_id: Uuid,
	workspace_id: Uuid,
	runner_id: Uuid,
	deployment_id: Uuid,
) {
	let session = serde_json::json!({
		"workspace_id": workspace_id,
		"runner_id": runner_id,
		"deployment_id": deployment_id,
	})
	.to_string();
	setup
		.set_redis_value(&format!("shellSession:{session_id}"), &session)
		.await;
}

/// Receive frames off the shell socket, skipping `Connecting` progress, until a
/// terminal `Error` arrives; assert its message contains `needle`.
async fn expect_error(ws: &mut TestWebSocket, needle: &str) {
	for _ in 0..10 {
		match ws.receive_json::<StreamDeploymentShellServerMsg>().await {
			StreamDeploymentShellServerMsg::Connecting { .. } => continue,
			StreamDeploymentShellServerMsg::Error { message } => {
				assert!(
					message.contains(needle),
					"expected an Error containing {needle:?}, got {message:?}"
				);
				return;
			}
			other => panic!("expected an Error frame, got {other:?}"),
		}
	}
	panic!("no Error frame received within 10 messages");
}

/// The migration must seed `deployment::shell` — otherwise `get_permission_id`
/// panics and every shell route 500s.
#[tokio::test]
async fn shell_permission_is_seeded() {
	let setup = setup().await.expect("failed to setup test server");
	// Panics if the permission isn't in the DB.
	let _ = setup.get_permission_id(Permission::Deployment(DeploymentPermission::Shell));
}

/// A user with no permission on the workspace can't open a shell — the
/// permission gate rejects it before any upgrade.
#[tokio::test]
async fn shell_without_permission_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	// A different user who is not a member of the workspace at all.
	let outsider = setup.create_test_user().await;

	let path = StreamDeploymentShellPath {
		workspace_id: workspace.id,
		deployment_id: deployment.id,
	}
	.to_string();
	let response = setup
		.open_web_dashboard_websocket(
			&path,
			StreamDeploymentShellRequestHeaders {
				authorization: outsider.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a user without deployment::shell must be rejected, got {}",
		response.status_code()
	);
}

/// Shelling into a stopped deployment fails loud with a clear message rather
/// than hanging.
#[tokio::test]
async fn shell_stopped_deployment_reports_error() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let path = StreamDeploymentShellPath {
		workspace_id: workspace.id,
		deployment_id: deployment.id,
	}
	.to_string();
	let mut ws = setup
		.open_web_dashboard_websocket(
			&path,
			StreamDeploymentShellRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;

	expect_error(&mut ws, "not running").await;
}

/// A running deployment whose runner has no live control socket fails fast,
/// without waiting for a dial-back.
#[tokio::test]
async fn shell_runner_not_connected_reports_error() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'running' WHERE id = '{}'",
			deployment.id
		))
		.await;

	let path = StreamDeploymentShellPath {
		workspace_id: workspace.id,
		deployment_id: deployment.id,
	}
	.to_string();
	let mut ws = setup
		.open_web_dashboard_websocket(
			&path,
			StreamDeploymentShellRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;

	expect_error(&mut ws, "not connected").await;
}

/// The runner is "connected" (lock present) but never dials back — the CLI side
/// gives up after the bounded timeout with a distinct message.
#[tokio::test]
async fn shell_dial_back_timeout_reports_error() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'running' WHERE id = '{}'",
			deployment.id
		))
		.await;
	// Fake a live runner control connection so the liveness pre-check passes.
	setup
		.set_redis_value(&format!("runnerConnectionLock:{}", runner.id), "1")
		.await;

	let path = StreamDeploymentShellPath {
		workspace_id: workspace.id,
		deployment_id: deployment.id,
	}
	.to_string();
	let mut ws = setup
		.open_web_dashboard_websocket(
			&path,
			StreamDeploymentShellRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;

	// The debug dial-back timeout is 3s.
	expect_error(&mut ws, "Timed out").await;
}

/// A runner must not be able to attach to a session bound to a *different*
/// runner, even with permission on its own runner — the hijack check rejects it
/// while the correctly-bound runner is allowed to upgrade.
#[tokio::test]
async fn shell_runner_dial_back_hijack_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner_a = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let runner_b = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner_a.id)
		.await;

	// A live session bound to runner A.
	let session_id = Uuid::new_v4();
	let session = serde_json::json!({
		"workspace_id": workspace.id,
		"runner_id": runner_a.id,
		"deployment_id": deployment.id,
	})
	.to_string();
	setup
		.set_redis_value(&format!("shellSession:{session_id}"), &session)
		.await;

	// Runner B (authorised for itself, super-admin token) dials the same
	// session — the binding mismatch must reject it.
	let hijack_path = StreamRunnerShellConnectionPath {
		workspace_id: workspace.id,
		runner_id: runner_b.id,
		session_id,
	}
	.to_string();
	let response = setup
		.open_web_dashboard_websocket(
			&hijack_path,
			StreamRunnerShellConnectionRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a runner attaching to another runner's session must be rejected, got {}",
		response.status_code()
	);

	// The correctly-bound runner A is allowed through the hijack check.
	let ok_path = StreamRunnerShellConnectionPath {
		workspace_id: workspace.id,
		runner_id: runner_a.id,
		session_id,
	}
	.to_string();
	let ws = setup
		.open_web_dashboard_websocket(
			&ok_path,
			StreamRunnerShellConnectionRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;
	ws.close().await;
}

/// RPOP a bridge list, retrying briefly since the handler pushes
/// asynchronously.
async fn poll_rpop(redis: &rustis::client::Client, key: &str) -> String {
	for _ in 0..50 {
		let popped: Vec<String> = redis.rpop(key, 1).await.unwrap();
		if let Some(frame) = popped.into_iter().next() {
			return frame;
		}
		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
	}
	panic!("no frame appeared on {key}");
}

/// End-to-end CLI-facing bridge: the handler signals the runner over the
/// control channel, relays the runner's `Connected`/`Output`/`Exit` down to the
/// CLI, and relays the CLI's stdin onto the `to-runner` list.
#[tokio::test]
async fn shell_bridge_relays_stdin_and_output() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'running' WHERE id = '{}'",
			deployment.id
		))
		.await;
	setup
		.set_redis_value(&format!("runnerConnectionLock:{}", runner.id), "1")
		.await;

	let redis = setup.state().redis.clone();

	// Subscribe to the control channel first so we catch the dial-back signal.
	let mut control = redis.create_pub_sub();
	control
		.subscribe(format!("{}/runner/{}/stream", workspace.id, runner.id))
		.await
		.unwrap();

	let path = StreamDeploymentShellPath {
		workspace_id: workspace.id,
		deployment_id: deployment.id,
	}
	.to_string();
	let mut ws = setup
		.open_web_dashboard_websocket(
			&path,
			StreamDeploymentShellRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;

	// The handler publishes ShellSessionRequested — pull the session id out.
	let signal = control.next().await.expect("signal").expect("redis ok");
	let payload: serde_json::Value = serde_json::from_slice(&signal.payload).unwrap();
	assert_eq!(payload["type"], "ShellSessionRequested");
	let session_id: Uuid = payload["session_id"].as_str().unwrap().parse().unwrap();

	let to_client = format!("shell:{session_id}:to-client");
	let to_runner = format!("shell:{session_id}:to-runner");

	// Act as the runner: publish the liveness beacon and signal Connected.
	setup
		.set_redis_value(&format!("shellRunnerAlive:{session_id}"), "1")
		.await;
	let _: usize = redis
		.lpush(
			&to_client,
			serde_json::to_string(&StreamDeploymentShellServerMsg::Connected).unwrap(),
		)
		.await
		.unwrap();

	loop {
		match ws.receive_json::<StreamDeploymentShellServerMsg>().await {
			StreamDeploymentShellServerMsg::Connecting { .. } => continue,
			StreamDeploymentShellServerMsg::Connected => break,
			other => panic!("expected Connected, got {other:?}"),
		}
	}

	// CLI stdin should land on `to-runner`.
	ws.send_json(&StreamDeploymentShellClientMsg::Stdin {
		data: b"ls\n".as_slice().into(),
	})
	.await;
	let stdin_frame = poll_rpop(&redis, &to_runner).await;
	let stdin: serde_json::Value = serde_json::from_str(&stdin_frame).unwrap();
	assert_eq!(stdin["type"], "Stdin");
	assert_eq!(stdin["data"], "bHMK"); // base64("ls\n")

	// Runner output should reach the CLI.
	let _: usize = redis
		.lpush(
			&to_client,
			serde_json::to_string(&StreamDeploymentShellServerMsg::Output {
				data: b"hi".as_slice().into(),
			})
			.unwrap(),
		)
		.await
		.unwrap();
	loop {
		if let StreamDeploymentShellServerMsg::Output { data } =
			ws.receive_json::<StreamDeploymentShellServerMsg>().await
		{
			assert_eq!(Vec::from(data), b"hi");
			break;
		}
	}

	// Runner exit ends the session.
	let _: usize = redis
		.lpush(
			&to_client,
			serde_json::to_string(&StreamDeploymentShellServerMsg::Exit { code: Some(0) }).unwrap(),
		)
		.await
		.unwrap();
	loop {
		if let StreamDeploymentShellServerMsg::Exit { code } =
			ws.receive_json::<StreamDeploymentShellServerMsg>().await
		{
			assert_eq!(code, Some(0));
			break;
		}
	}
}

/// The runner-facing backpressure gate: when the CLI side never drains, the
/// `to-client` queue plateaus at the high-water mark instead of growing
/// unbounded, and the producer resumes once the queue drains.
#[tokio::test]
async fn shell_runner_backpressure_plateaus() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let session_id = Uuid::new_v4();
	seed_session(&setup, session_id, workspace.id, runner.id, deployment.id).await;

	let redis = setup.state().redis.clone();
	let to_client = format!("shell:{session_id}:to-client");

	let path = StreamRunnerShellConnectionPath {
		workspace_id: workspace.id,
		runner_id: runner.id,
		session_id,
	}
	.to_string();
	let mut ws = setup
		.open_web_dashboard_websocket(
			&path,
			StreamRunnerShellConnectionRequestHeaders {
				authorization: user.access_token.clone(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.into_websocket()
		.await;

	// Blast output well past the high-water mark (256); nobody drains to-client.
	let produced = 400;
	for _ in 0..produced {
		ws.send_json(&StreamRunnerShellConnectionClientMsg::Output {
			data: vec![b'x'; 64].into(),
		})
		.await;
	}
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	let len: usize = redis.llen(&to_client).await.unwrap();
	assert!(
		len < produced,
		"backpressure must cap the queue below the produced count ({produced}), got {len}"
	);
	assert!(
		len >= 200,
		"queue should fill to ~high-water (256) before the gate pauses, got {len}"
	);

	// Drain well below the low-water mark; the producer should resume.
	for _ in 0..(produced) {
		let _: Vec<String> = redis.rpop(&to_client, 1).await.unwrap();
	}
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	let refilled: usize = redis.llen(&to_client).await.unwrap();
	assert!(
		refilled > 0,
		"producer should resume reading the buffered frames after the queue drains, got {refilled}"
	);

	ws.close().await;
}
