pub use crate::prelude::*;
use crate::{utils::ApiRequest, ApiEndpoint};

pub enum AuthenticationType<E>
where
	E: ApiEndpoint,
{
	NoAuthentication,
	PlainTokenAuthenticator,
	WorkspaceMembershipAuthenticator {
		extract_workspace_id: fn(&ApiRequest<E>) -> Uuid,
	},
	ResourcePermissionAuthenticator {
		extract_resource_id: fn(&ApiRequest<E>) -> Uuid,
		// permission: ResourcePermission,
	},
}
