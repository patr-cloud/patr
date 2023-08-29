pub use crate::prelude::*;
use crate::{api::EmptyRequest, utils::ApiRequest, ApiEndpoint};

pub enum AuthenticatorType<E = EmptyRequest>
where
	E: ApiEndpoint,
{
	NoAuthentication,
	PlainTokenAuthenticator,
	WorkspaceMembershipAuthenticator {
		extract_workspace_id: fn(&ApiRequest<E>) -> Uuid,
	},
}
