use crate::prelude::*;

macros::declare_stream_endpoint!(
	/// Open an interactive shell inside a running deployment. The client (the
	/// CLI) streams stdin and terminal resize events up, and receives the
	/// container's output back, until the shell exits or the session errors.
	StreamDeploymentShell,
	GET "/workspace/{workspace_id}/deployment/{deployment_id}/shell" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to open the shell into
		pub deployment_id: Uuid,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::Shell),
		}
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	server_msg = {
		/// Human-readable progress emitted while the session is being set up
		/// (locating the deployment, contacting the runner, etc.), so the user
		/// sees why they're waiting instead of a frozen cursor.
		Connecting {
			/// The progress message to show the user
			message: String,
		},
		/// The end-to-end bridge into the container is live; the raw shell
		/// takes over after this frame.
		Connected,
		/// A chunk of output from the container (stdout/stderr are merged under
		/// the TTY). Carried as base64 to avoid the per-byte JSON blowup of a
		/// `Vec<u8>`.
		Output {
			/// The output bytes, base64 encoded
			data: Base64String,
		},
		/// The shell process exited normally with this exit code (if known).
		Exit {
			/// The exit code of the shell process, if the runner could
			/// determine it
			code: Option<i32>,
		},
		/// The session ended abnormally (runner offline, dial-back timeout,
		/// exec failure, mid-session disconnect, ...). The message is safe to
		/// show the user.
		Error {
			/// A human-readable description of what went wrong
			message: String,
		},
	},
	client_msg = {
		/// A chunk of stdin from the user's terminal, base64 encoded.
		Stdin {
			/// The stdin bytes, base64 encoded
			data: Base64String,
		},
		/// The user's terminal was resized; the container's PTY should follow.
		Resize {
			/// The number of rows in the terminal
			rows: u16,
			/// The number of columns in the terminal
			cols: u16,
		},
	},
	audit_log = NoAuditLogger,
);

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	// The wire format is the contract the TS ws client and the runner both
	// depend on: `#[serde(tag = "type")]`, PascalCase variant names, snake_case
	// fields, and terminal bytes as a base64 string.

	#[test]
	fn server_output_wire_format() {
		let msg = StreamDeploymentShellServerMsg::Output {
			data: b"hello".as_slice().into(),
		};
		assert_eq!(
			serde_json::to_value(&msg).unwrap(),
			json!({ "type": "Output", "data": "aGVsbG8=" })
		);
	}

	#[test]
	fn server_connecting_and_terminal_frames() {
		assert_eq!(
			serde_json::to_value(StreamDeploymentShellServerMsg::Connecting {
				message: "Contacting runner".to_owned(),
			})
			.unwrap(),
			json!({ "type": "Connecting", "message": "Contacting runner" })
		);
		assert_eq!(
			serde_json::to_value(StreamDeploymentShellServerMsg::Connected).unwrap(),
			json!({ "type": "Connected" })
		);
		assert_eq!(
			serde_json::to_value(StreamDeploymentShellServerMsg::Exit { code: Some(0) }).unwrap(),
			json!({ "type": "Exit", "code": 0 })
		);
	}

	#[test]
	fn client_stdin_and_resize_wire_format() {
		assert_eq!(
			serde_json::to_value(StreamDeploymentShellClientMsg::Stdin {
				data: vec![0x00, 0xff, 0x1b].into(),
			})
			.unwrap(),
			json!({ "type": "Stdin", "data": "AP8b" })
		);
		assert_eq!(
			serde_json::to_value(StreamDeploymentShellClientMsg::Resize {
				rows: 40,
				cols: 120
			})
			.unwrap(),
			json!({ "type": "Resize", "rows": 40, "cols": 120 })
		);
	}

	#[test]
	fn stdin_round_trips_non_utf8_bytes() {
		let original = vec![0u8, 159, 146, 150, 27, 255];
		let msg = StreamDeploymentShellClientMsg::Stdin {
			data: original.clone().into(),
		};
		let wire = serde_json::to_string(&msg).unwrap();
		let StreamDeploymentShellClientMsg::Stdin { data } = serde_json::from_str(&wire).unwrap()
		else {
			panic!("expected Stdin");
		};
		assert_eq!(Vec::from(data), original);
	}
}
