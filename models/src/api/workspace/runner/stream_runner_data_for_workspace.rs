use crate::{api::workspace::deployment::Deployment, prelude::*};

macros::declare_stream_endpoint!(
	/// Subscribe to the changes for a particular runner in a workspace
	StreamRunnerDataForWorkspace,
	GET "/workspace/:workspace_id/runner/:runner_id/stream" {
		/// The workspace the runners belongs to
		pub workspace_id: Uuid,
		/// The runner to subscribe to
		pub runner_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id
		}
	},
	server_msg = {
		/// The user has created a new deployment on their account
		DeploymentCreated(WithId<Deployment>),
		/// The user has updated a deployment on their account
		DeploymentUpdated {
			/// The ID of the deployment that was updated
			id: Uuid,
			/// The old deployment data
			old: Deployment,
			/// The new deployment data
			new: Deployment,
		},
		/// The user has deleted a deployment on their account
		DeploymentDeleted(Uuid),
		/// The user has requested a ping, and the server is responding with a pong
		Pong,
	},
	client_msg = {
		/// The user is pinging the server
		Ping,
	},
);
