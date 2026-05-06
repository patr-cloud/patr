use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to validate user's entered email ID is available or not
	IsDomainValid,
	GET "/domain/is-valid",
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The domain that has to be verified
		#[preprocess(trim, domain)]
		pub domain: String,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Domain(DomainPermission::Add),
		}
	},
	response = {
		/// A boolean response corresponding to the validity of the domain
		pub valid: bool,
	},
	audit_log = NoAuditLogger,
);
