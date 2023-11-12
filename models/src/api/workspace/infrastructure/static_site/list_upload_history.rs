use super::StaticSiteUploadHistory;
use crate::{
	prelude::*,
	utils::{BearerToken, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to get all upload history of a static site
	ListStaticSiteUploadHistory,
	GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID to get history of
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
	pagination = true,
	response = {
		/// The list of uplaod history which contains:
		/// upload_id - The ID of the upload
		/// message - The release message of the upload
		/// uploaded_by - The ID of the user who uploaded
		/// created - The date and time when the upload was created
		/// processed - The data and time when the static site was updated
		pub uploads: Vec<WithId<StaticSiteUploadHistory>>
	}
);
