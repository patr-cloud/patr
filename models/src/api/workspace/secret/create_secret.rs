use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a secret
	CreateSecret,
	POST "/workspace/:workspace_id/secret" {
		/// The ID of the workspace
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
			permission: Permission::Secret(SecretPermission::Create),
		}
	},
	request = {
		/// The name of the secret
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The value of the secret, i.e, the secret content
		#[preprocess(none)]
		pub value: String,
	},
	response = {
		/// The ID of the created secret
		#[serde(flatten)]
		pub id:  WithId<()>
	}
);
