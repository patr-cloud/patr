/// Approve a runner consent link from the browser
mod approve_runner_link;
/// Begin a runner consent-link flow from the CLI
mod create_runner_link;
/// Get the token to use for the ingress tunnel for this runner
mod get_ingress_token_for_runner;
/// The endpoint to get the details of a runner in a workspace
mod get_runner_info;
/// Fetch metadata for a runner consent link (consent page)
mod get_runner_link;
/// The endpoint to get the logs of a runner process
mod get_runner_logs;
/// The endpoint to get system metrics for a runner
mod get_runner_metrics;
/// The endpoint to list all the runners in a workspace
mod list_runners_for_workspace;
/// Reconnect an existing runner to a new machine (rotate its SA token)
mod reconnect_runner_link;
/// The endpoint to remove a runner from a workspace
mod remove_runner_from_workspace;
/// The endpoint to stream the runner data for a workspace
mod stream_runner_data_for_workspace;
/// The endpoint to stream runner process logs in real time
mod stream_runner_logs;
/// Poll a runner consent link from the CLI
mod verify_runner_link;

use semver::Version;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

pub use self::{
	approve_runner_link::*,
	create_runner_link::*,
	get_ingress_token_for_runner::*,
	get_runner_info::*,
	get_runner_link::*,
	get_runner_logs::*,
	get_runner_metrics::*,
	list_runners_for_workspace::*,
	reconnect_runner_link::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
	stream_runner_logs::*,
	verify_runner_link::*,
};
use crate::prelude::*;

/// Represents a runner for a Patr workspace.
///
/// A runner is basically what runs the deployments, databases, etc for a
/// workspace. A runner connects to the Patr API and listens for commands to
/// run. Since runners are long-lived processes, they can be disconnected and
/// reconnected at any time. This struct represents the state of a runner. Since
/// runners are arbitrary code that executes the deployments, they can execute
/// the deployments in any way they want. This includes running the deployments
/// on a VM, kubernetes, or even on other `PaaS` providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
	/// The name of the runner
	pub name: String,
	/// Whether the runner is connected to the Patr API currently or not
	pub connected: bool,
	/// The last timestamp the runner was seen online
	#[ts(type = "Date | null")]
	pub last_seen: Option<OffsetDateTime>,
	/// The semver version the runner last reported on its handshake. For
	/// runners that existed before version reporting, this is `0.0.0` until
	/// they reconnect.
	#[ts(type = "string")]
	#[search(skip)]
	pub version: Version,
}

/// A single log entry from a runner process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct RunnerLog {
	/// Timestamp of the log entry
	#[ts(type = "Date")]
	pub timestamp: OffsetDateTime,
	/// The log message
	pub log: String,
}

/// The set of available per-runner metric names. Used as a path parameter
/// in the metrics endpoint to select which metric to query.
#[derive(
	Debug,
	Clone,
	Serialize,
	Deserialize,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	TS,
	strum::Display,
	strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RunnerMetricName {
	/// System CPU usage percentage
	SystemCpuUsage,
	/// System memory usage percentage
	SystemMemoryUsage,
	/// Disk read rate in bytes per second
	SystemDiskReadBytes,
	/// Disk write rate in bytes per second
	SystemDiskWrittenBytes,
	/// Disk usage percentage
	SystemDiskUsage,
	/// Network receive rate in bytes per second
	SystemNetworkRx,
	/// Network transmit rate in bytes per second
	SystemNetworkTx,
}
