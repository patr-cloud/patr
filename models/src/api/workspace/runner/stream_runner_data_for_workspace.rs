use std::net::IpAddr;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
	api::workspace::{
		deployment::{Deployment, DeploymentRunningDetails, DeploymentStatus},
		managed_url::ManagedUrl,
	},
	prelude::*,
};

/// This enum represents how the Runner will expose the resources to the
/// outside world. This is used to determine how the Runner will handle the
/// resources, such as whether it will use a tunnel, or whether it will
/// expose the resources directly, or if each resource has it's own exposed URL
/// on it's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RunnerExposureType {
	/// The runner will need to expose the resources through a tunnel, and run a
	/// reverse proxy to the resources.
	Private,
	/// The runner has a public IP address, and the resources will be exposed
	/// through a reverse proxy. This runner will not expose the resources
	/// through a tunnel, but will run a reverse proxy to the resources.
	#[serde(rename_all = "camelCase")]
	PublicIP {
		/// The public IP address(es) of the runner. This is what will be used
		/// as the DNS record.
		ip_addresses: Vec<IpAddr>,
	},
	/// The runner has a public DNS name, and the resources will be exposed
	/// through a reverse proxy. This runner will not expose the resources
	/// through a tunnel, but will run a reverse proxy to the resources.
	#[serde(rename_all = "camelCase")]
	PublicDNS {
		/// The public DNS name of the runner. This is what will be used as the
		/// CNAME DNS record.
		dns_name: String,
	},
}

impl RunnerExposureType {
	/// Returns true if the runner is a private runner, meaning it will
	/// expose the resources through a tunnel, and run a reverse proxy to the
	/// resources.
	#[must_use]
	pub fn is_private(&self) -> bool {
		matches!(self, RunnerExposureType::Private)
	}

	/// Returns true if the runner is a public runner, meaning it has a public
	/// IP address or a public DNS name, and will expose the resources through
	/// a reverse proxy.
	#[must_use]
	pub fn is_public(&self) -> bool {
		matches!(
			self,
			RunnerExposureType::PublicIP { .. } | RunnerExposureType::PublicDNS { .. }
		)
	}

	/// Returns true if the runner needs to run a tunnel to expose the
	/// resources. This is true for private runners, which will run a tunnel
	/// to expose the resources, and false for public runners, which will not
	/// run a tunnel.
	#[must_use]
	pub fn requires_tunnel(&self) -> bool {
		matches!(self, RunnerExposureType::Private)
	}
}

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
			permission: Permission::Runner(RunnerPermission::Execute),
		}
	},
	client_type = [ServiceAccount],
	server_msg = {
		/// The runner needs to send its handshake before proceeding
		HandshakeRequired,
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
		/// The user has created a managed URL that targets a deployment on
		/// this runner. Only `ProxyDeployment` URLs are streamed today.
		///
		/// `managed_url` is intentionally *not* flattened: `ManagedUrl`
		/// flattens `ManagedUrlType` whose discriminator is also `"type"`,
		/// which would collide with this enum's `tag = "type"` and produce
		/// JSON with a duplicate `"type"` key — serialise OK, deserialise
		/// fails.
		ManagedUrlCreated {
			/// The managed URL that was created
			managed_url: WithId<ManagedUrl>,
		},
		/// The user has updated a managed URL on this runner. Sent when the
		/// existing URL stays a `ProxyDeployment` for a deployment on this
		/// runner; transitions in/out of `ProxyDeployment` come through as a
		/// pair of `ManagedUrlCreated`/`ManagedUrlDeleted` instead.
		///
		/// See `ManagedUrlCreated` for why `managed_url` is not flattened.
		ManagedUrlUpdated {
			/// The managed URL after the update
			managed_url: WithId<ManagedUrl>,
		},
		/// A managed URL on this runner was deleted, or transitioned away
		/// from `ProxyDeployment` for this runner.
		ManagedUrlDeleted {
			/// The ID of the managed URL that was deleted
			id: Uuid,
		},
	},
	client_msg = {
		/// Initial handshake sent by the runner immediately after the WebSocket
		/// upgrade. The runner reports metadata about itself that the API needs
		/// before it can stream state (version for outdated detection, exposure
		/// type for ingress planning).
		Handshake {
			/// The semver version of the runner binary
			version: Version,
			/// The exposure type for the runner
			exposure_type: RunnerExposureType,
		},
		/// A deployment has updated with the following new status
		DeploymentStatusUpdated {
			/// The ID of the deployment that was updated
			id: Uuid,
			/// The new status of the deployment
			status: DeploymentStatus,
		},
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.runner_id),
	},
);
