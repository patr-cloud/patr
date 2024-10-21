mod auth;
mod profile;
mod workspace;

mod logged_in_routes;
mod not_workspaced_content;
mod workspaced_content;

use serde::{Deserialize, Serialize};

pub use self::{
	auth::*,
	logged_in_routes::*,
	not_workspaced_content::*,
	profile::*,
	workspace::*,
	workspaced_content::*,
};

/// The direction of the sort
#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum SortDirection {
	/// Sort in ascending order
	Ascending,
	/// Sort in descending order
	Descending,
}

/// Common query parameters for all routes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommonQueryParams {
	/// The Page number to fetch
	#[serde(skip_serializing_if = "Option::is_none")]
	pub page: Option<u32>,
	/// The Sorting order of the resource
	#[serde(skip_serializing_if = "Option::is_none")]
	pub order: Option<SortDirection>,
	/// Filter the resource by the given query
	#[serde(skip_serializing_if = "Option::is_none")]
	pub query: Option<String>,
}
