/// The endpoint to add a runner to a workspace
mod add_runner_to_workspace;
/// The endpoint to get the details of a runner in a workspace
mod get_runner_info;
/// The endpoint to list all the runners in a workspace
mod list_runners_for_workspace;
/// The endpoint to remove a runner from a workspace
mod remove_runner_from_workspace;
/// The endpoint to stream the runner data for a workspace
mod stream_runner_data_for_workspace;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use self::{
	add_runner_to_workspace::*,
	get_runner_info::*,
	list_runners_for_workspace::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
};

/// Represents a runner for a Patr workspace.
///
/// A runner is basically what runs the deployments, databases, etc for a
/// workspace. A runner connects to the Patr API and listens for commands to
/// run. Since runners are long-lived processes, they can be disconnected and
/// reconnected at any time. This struct represents the state of a runner. Since
/// runners are arbitrary code that executes the deployments, they can execute
/// the deployments in any way they want. This includes running the deployments
/// on a VM, kubernetes, or even on other PaaS providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
	/// The name of the runner
	pub name: String,
	/// Whether the runner is connected to the Patr API currently or not
	pub connected: bool,
	/// The last timestamp the runner was seen online
	pub last_seen: Option<OffsetDateTime>,
}
