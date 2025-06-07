use super::StaticSiteUploadHistory;
use crate::prelude::*;

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
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.static_site_id,
			permission: Permission::StaticSite(StaticSitePermission::View),
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
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
