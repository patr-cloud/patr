use crate::prelude::*;

macros::declare_stream_endpoint!(
	/// The per-session websocket the runner dials back to after the API signals
	/// `ShellSessionRequested` over the control socket. The runner is the
	/// websocket *client* here, so the directions are inverted relative to
	/// [`super::super::deployment::StreamDeploymentShell`]: the API sends stdin
	/// and resize events down (`server_msg`), and the runner sends the
	/// container's output back up (`client_msg`).
	StreamRunnerShellConnection,
	GET "/workspace/{workspace_id}/runner/{runner_id}/shell/{session_id}" {
		/// The workspace the runner belongs to
		pub workspace_id: Uuid,
		/// The runner opening the shell session
		pub runner_id: Uuid,
		/// The session ID minted by the API when the CLI connected. Used to
		/// look up and bind the session; not a security boundary on its own
		/// (see the hijack check in the handler).
		pub session_id: Uuid,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Execute),
		}
	},
	request_headers = {
		/// Token used to authorize the runner
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	server_msg = {
		/// A chunk of stdin to feed the exec, base64 encoded.
		Stdin {
			/// The stdin bytes, base64 encoded
			data: Base64String,
		},
		/// The user's terminal was resized; the exec's PTY should follow.
		Resize {
			/// The number of rows in the terminal
			rows: u16,
			/// The number of columns in the terminal
			cols: u16,
		},
		/// The CLI side went away; the runner should terminate the exec.
		Close,
	},
	client_msg = {
		/// A chunk of output from the container (stdout/stderr merged under the
		/// TTY), base64 encoded.
		Output {
			/// The output bytes, base64 encoded
			data: Base64String,
		},
		/// The exec exited with this code (if the runner could determine it).
		Exit {
			/// The exit code of the shell process, if known
			code: Option<i32>,
		},
		/// The runner failed to open or run the exec (no running container, no
		/// shell in the image, ...). The message is relayed to the CLI.
		Error {
			/// A human-readable description of what went wrong
			message: String,
		},
	},
	audit_log = NoAuditLogger,
);

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn server_to_runner_wire_format() {
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionServerMsg::Stdin {
				data: b"ls\n".as_slice().into(),
			})
			.unwrap(),
			json!({ "type": "Stdin", "data": "bHMK" })
		);
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionServerMsg::Resize {
				rows: 24,
				cols: 80
			})
			.unwrap(),
			json!({ "type": "Resize", "rows": 24, "cols": 80 })
		);
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionServerMsg::Close).unwrap(),
			json!({ "type": "Close" })
		);
	}

	#[test]
	fn client_from_runner_wire_format() {
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionClientMsg::Output {
				data: b"done".as_slice().into(),
			})
			.unwrap(),
			json!({ "type": "Output", "data": "ZG9uZQ==" })
		);
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionClientMsg::Exit { code: Some(137) })
				.unwrap(),
			json!({ "type": "Exit", "code": 137 })
		);
		assert_eq!(
			serde_json::to_value(StreamRunnerShellConnectionClientMsg::Error {
				message: "no shell".to_owned()
			})
			.unwrap(),
			json!({ "type": "Error", "message": "no shell" })
		);
	}
}
