use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Which build of Patr this API server is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DeploymentType {
	/// The hosted SaaS at `patr.cloud`.
	Cloud,
	/// An operator-run instance.
	SelfHosted,
}

macros::declare_api_endpoint!(
	/// Static information about this API instance. Clients hit this
	/// unauthenticated on first load to discover the deployment type and any
	/// instance-specific values they can't know at build time.
	GetApiEnvironment,
	GET "/info",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// The version of the API server (the `CARGO_PKG_VERSION` of the
		/// `api` crate at build time).
		pub version: String,
		/// Whether this is the cloud or a self-hosted deployment.
		pub deployment_type: DeploymentType,
		/// The base domain the API is served on. Only present on
		/// self-hosted, where the operator picks it; cloud clients hard-code
		/// `patr.cloud` at build time.
		#[serde(skip_serializing_if = "Option::is_none")]
		pub base_domain: Option<String>,
	},
	audit_log = NoAuditLogger,
);
