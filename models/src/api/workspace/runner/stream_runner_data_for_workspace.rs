use crate::{
	api::workspace::deployment::{Deployment, DeploymentRunningDetails, DeploymentStatus},
	prelude::*,
	rbac::ResourceType,
};

macros::declare_stream_endpoint!(
	/// Subscribe to the changes for a particular runner in a workspace
	StreamRunnerDataForWorkspace,
	GET "/workspace/{workspace_id}/runner/{runner_id}/stream" {
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
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::View),
		}
	},
	server_msg = {
		/// The user has created a new deployment on their account
		DeploymentCreated {
			/// The deployment that was created
			#[serde(flatten)]
			deployment: WithId<Deployment>,
			/// The running details of the deployment that was created
			#[serde(flatten)]
			running_details: DeploymentRunningDetails,
		},
		/// The user has updated a deployment on their account
		DeploymentUpdated {
			/// The details of the deployment after the update
			#[serde(flatten)]
			deployment: WithId<Deployment>,
			/// The running details of the deployment that was created
			#[serde(flatten)]
			running_details: DeploymentRunningDetails,
		},
		/// The user has deleted a deployment on their account
		DeploymentDeleted {
			/// The ID of the deployment that was deleted
			id: Uuid
		},
	},
	client_msg = {
		/// A deployment has updated with the following new status
		DeploymentStatusUpdated {
			/// The ID of the deployment that was updated
			id: Uuid,
			/// The new status of the deployment
			status: DeploymentStatus,
		},
	},
);

impl StreamRunnerDataForWorkspaceServerMsg {
	/// Get the resource type that this message is related to
	#[must_use]
	pub fn resource_type(&self) -> ResourceType {
		match self {
			Self::DeploymentCreated { .. } |
			Self::DeploymentUpdated { .. } |
			Self::DeploymentDeleted { .. } => ResourceType::Deployment,
		}
	}
}
