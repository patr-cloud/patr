use crate::{
	prelude::*,
	utils::{BearerToken, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to delete a static site
	/// This route will permenantly delete the static site including it's history
	/// and the current index.html file
	DeleteStaticSite,
	DELETE "/workspace/:workspace_id/infrastructure/static-site/:static_site_id" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID to be deleted
		pub static_site_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.static_site_id
		}
	}
);
