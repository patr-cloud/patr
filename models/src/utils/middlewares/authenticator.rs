use std::future::Future;

pub use crate::prelude::*;
use crate::{api::EmptyRequest, utils::ApiRequest, ApiEndpoint};

pub enum AuthenticatorMiddleware<E = EmptyRequest>
where
	E: ApiEndpoint,
{
	NoAuthentication,
	PlainTokenAuthenticator,
	WorkspaceMembershipAuthenticator {
		extract_workspace_id: fn(&ApiRequest<E>) -> dyn Future<Output = Uuid>,
	},
}
