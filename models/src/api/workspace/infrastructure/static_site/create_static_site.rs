use super::StaticSiteDetails;
use crate::prelude::*;

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
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The static site name
		#[preprocess(trim, lowercase, regex = "^[a-zA-Z0-9_\\-\\.][a-zA-Z0-9_\\-\\. ]{0,62}[a-zA-Z0-9_\\-\\.]$")]
		pub name: String,
		/// Release message (eg: v1.0.0)
		#[preprocess(trim, lowercase)]
		pub message: String,
		/// The static site index.html file
		#[preprocess(trim, lowercase)]
		pub file: Option<String>,
		/// Static site details which included metrics, etc
		#[preprocess(none)]
		pub static_site_details: StaticSiteDetails,
	},
	response = {
		/// The new static site ID
		#[serde(flatten)]
		pub id: WithId<()>
	}
);
