use super::StaticSiteDetails;
use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Definition of a route to create a new static site
	/// This route will allow users to upload a new index.html which would go live
	CreateStaticSite,
	POST "/workspace/:workspace_id/infrastructure/static-site" {
		/// The workspace ID of the user
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::StaticSite(StaticSitePermission::Create),
		}
	},
	request = {
		/// The static site name
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// Release message (eg: v1.0.0)
		pub message: String,
		/// The static site index.html file
		#[preprocess(none)]
		pub file: Option<String>,
		/// Static site details which included metrics, etc
		#[preprocess(none)]
		pub static_site_details: StaticSiteDetails,
	},
	response = {
		/// The new static site ID
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
