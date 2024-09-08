use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// The endpoint to create a static site
mod create_static_site;
/// The endpoint to delete a static site
mod delete_static_site;
/// The endpoint to get the details of a static site
mod get_static_site_info;
/// The endpoint to list all the static sites in a workspace
mod list_static_site;
/// The endpoint to list the upload history of a static site
mod list_upload_history;
/// The endpoint to revert a static site to a previous upload
mod revert_static_site;
/// The endpoint to start a static site
mod start_static_site;
/// The endpoint to stop a static site
mod stop_static_site;
/// The endpoint to update the details of a static site
mod update_static_site;
/// The endpoint to upload a new version of a static site
mod upload_static_site;

pub use self::{
	create_static_site::*,
	delete_static_site::*,
	get_static_site_info::*,
	list_static_site::*,
	list_upload_history::*,
	revert_static_site::*,
	start_static_site::*,
	stop_static_site::*,
	update_static_site::*,
	upload_static_site::*,
};
use super::deployment::DeploymentStatus;
use crate::utils::Uuid;

/// Static site
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSite {
	/// Name of the static site
	pub name: String,
	/// Status of the static site.
	///
	/// Can either be:
	/// - Created,
	/// - Pushed,
	/// - Deploying,
	/// - Running,
	/// - Stopped,
	/// - Errored,
	/// - Deleted,
	pub status: DeploymentStatus,
	/// The current index.html file running
	pub current_live_upload: Option<Uuid>, /* add more details about the
	                                        * static site */
}

/// Static site details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteDetails {
	// add more details here, like metrics, etc.
}

/// Static site upload history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteUploadHistory {
	/// The upload ID
	pub upload_id: Uuid,
	/// The release message the static site was uploaded with
	pub message: String,
	/// The user ID of the user who uploaded
	pub uploaded_by: Uuid,
	/// The timestamp of when the static site was created
	pub created: OffsetDateTime,
	/// The timestamp of when the static site was processed
	pub processed: Option<OffsetDateTime>,
}
