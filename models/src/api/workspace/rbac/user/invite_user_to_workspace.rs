use crate::{api::workspace::rbac::user::RoleBindingGrant, prelude::*};

macros::declare_api_endpoint!(
	/// Route to invite a user, by email address, to a workspace with a given set
	/// of roles. The invitee receives an email with a link to accept the invite.
	/// If they already have a Patr account they can accept directly, otherwise
	/// they sign up first and then accept.
	InviteUserToWorkspace,
	POST "/workspace/{workspace_id}/rbac/user/invite" {
		/// The ID of the workspace to invite the user to
		pub workspace_id: Uuid,
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
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ModifyRoles,
		}
	},
	api = false,
	request = {
		/// The email address to invite to the workspace
		#[preprocess(trim, lowercase, email)]
		pub email: String,
		/// The role grants the invitee receives once they accept
		#[preprocess(none)]
		pub roles: Vec<RoleBindingGrant>,
	},
	response = {
		/// The ID of the created invite
		#[serde(flatten)]
		pub id: OnlyId,
		/// The accept link for this invite, containing the plaintext token. This
		/// is the only time the token is returned (it is stored hashed), so the
		/// caller can offer a "copy link" affordance right after inviting.
		pub accept_url: String,
	},
	audit_log = NoAuditLogger,
);
