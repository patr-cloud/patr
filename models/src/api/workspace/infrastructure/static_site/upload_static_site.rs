use crate::{
	prelude::*,
	utils::{Uuid, BearerToken},
}; 

macros::declare_api_endpoint!(
	/// Route to upload to a static site
	/// This route will upload a new index.html file which would go live
	UploadStaticSite,
	POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID of static site to upload index.html file
		pub static_site_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
			extract_resource_id: |req| req.path.static_site_id
		}
	},
	request = {
		/// The new index.html file
		pub file: String,
		/// The release note (eg: v1.0.0)
		pub message: String
	},
	response = {
		/// The upload ID of the new upload
		#[serde(flatten)]
		pub upload_id: WithId<()>
	}
);