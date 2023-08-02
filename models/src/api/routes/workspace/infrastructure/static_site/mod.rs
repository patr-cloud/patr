use chrono::Utc;
use serde::{Deserialize, Serialize};

mod create_static_site;
mod delete_static_site;
mod get_static_site_info;
mod list_linked_urls;
mod list_static_sites;
mod list_upload_history;
mod revert_static_site;
mod start_static_site;
mod stop_static_site;
mod update_static_site;
mod upload_static_site;

pub use self::{
	create_static_site::*,
	delete_static_site::*,
	get_static_site_info::*,
	list_linked_urls::*,
	list_static_sites::*,
	list_upload_history::*,
	revert_static_site::*,
	start_static_site::*,
	stop_static_site::*,
	update_static_site::*,
	upload_static_site::*,
};
use super::deployment::DeploymentStatus;
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSite {
	pub id: Uuid,
	pub name: String,
	pub status: DeploymentStatus,
	pub current_live_upload: Option<Uuid>, /* add more details about the
	                                        * static site */
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteDetails {
	// add more details here, like metrics, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteUploadHistory {
	pub upload_id: Uuid,
	pub message: String,
	pub uploaded_by: Uuid,
	pub created: DateTime<Utc>,
	pub processed: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{StaticSite, StaticSiteDetails, StaticSiteUploadHistory};
	use crate::{
		models::workspace::infrastructure::deployment::DeploymentStatus,
		utils::{DateTime, Uuid},
	};

	#[test]
	fn assert_static_site_types() {
		assert_tokens(
			&StaticSite {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "John Patr's static site".to_string(),
				status: DeploymentStatus::Created,
				current_live_upload: Some(
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
						.unwrap(),
				),
			},
			&[
				Token::Struct {
					name: "StaticSite",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's static site"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "created",
				},
				Token::Str("currentLiveUpload"),
				Token::Some,
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_static_site_details_types() {
		assert_tokens(
			&StaticSiteDetails {},
			&[
				Token::Struct {
					name: "StaticSiteDetails",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_static_site_deploy_history_types() {
		assert_tokens(
			&StaticSiteUploadHistory {
				upload_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				message: "v2".to_string(),
				uploaded_by: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				created: DateTime::from_str("2020-04-12 22:10:57+02:00")
					.unwrap(),
				processed: Some(
					DateTime::from_str("2020-04-12 22:10:57+02:00").unwrap(),
				),
			},
			&[
				Token::Struct {
					name: "StaticSiteUploadHistory",
					len: 5,
				},
				Token::Str("uploadId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("message"),
				Token::Str("v2"),
				Token::Str("uploadedBy"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("processed"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::StructEnd,
			],
		)
	}
}
