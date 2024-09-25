mod auth;
mod profile;
mod workspace;

mod logged_in_routes;
mod logged_out_routes;
mod not_workspaced_content;
mod workspaced_content;

use models::prelude::*;
use serde::{Deserialize, Serialize};

pub use self::{
	auth::*,
	logged_in_routes::*,
	logged_out_routes::*,
	not_workspaced_content::*,
	profile::*,
	workspace::*,
	workspaced_content::*,
};

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum SortDirection {
	Ascending,
	Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommonQueryParams {
	/// The Page number to fetch
	#[serde(skip_serializing_if = "Option::is_none")]
	pub page: Option<u32>,
	/// The Sorting order of the deployments
	#[serde(skip_serializing_if = "Option::is_none")]
	pub order: Option<SortDirection>,
	/// Filter the deployments by date
	#[serde(skip_serializing_if = "Option::is_none")]
	pub query: Option<OneOrMore<String>>,
}
