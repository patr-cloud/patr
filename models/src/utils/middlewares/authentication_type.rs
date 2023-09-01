use crate::prelude::*;

/// This enum represents the different types of authentication that can be used
/// for an API endpoint.
///
/// The variants are:
/// - [`NoAuthentication`][1]: No authentication is required for this endpoint.
/// - [`PlainTokenAuthenticator`][2]: Any logged in user can access this
///   endpoint.
/// - [`WorkspaceMembershipAuthenticator`][3]: Only users that are members of
///   the workspace that is specified in the request can access this endpoint.
/// - [`ResourcePermissionAuthenticator`][4]: Only users that have the specified
///   permission on the resource that is specified in the request can access
///   this endpoint.
///
/// This enum is used in the [`ApiEndpoint`] trait to specify the authentication
/// type of an endpoint. The constant in the trait is used by the router
/// extension to mount the corresponding [`tower::Layer`] in the router.
///
/// [1]: AuthenticationType::<E>::NoAuthentication
/// [2]: AuthenticationType::<E>::PlainTokenAuthenticator
/// [3]: AuthenticationType::<E>::WorkspaceMembershipAuthenticator
/// [4]: AuthenticationType::<E>::ResourcePermissionAuthenticator
pub enum AuthenticationType<E>
where
	E: ApiEndpoint,
{
	/// No authentication is required for this endpoint.
	NoAuthentication,
	/// Any logged in user can access this endpoint.
	PlainTokenAuthenticator,
	/// Only users that are members of the workspace that is specified in the
	/// [`extract_workspace_id`][1] function can access this endpoint.
	///
	/// [1]: AuthenticationType::<E>::WorkspaceMembershipAuthenticator::extract_workspace_id
	WorkspaceMembershipAuthenticator {
		/// This function is used to extract the workspace id from the request.
		/// The workspace id is then used to check if the user is a member of
		/// the workspace.
		extract_workspace_id: fn(&ApiRequest<E>) -> Uuid,
	},
	/// Only users that have the permission specific by [`permission`][2] on the resource that is
	/// specified in the [`extract_resource_id`][1] function can access this
	/// endpoint.
	/// 
	/// [1]: AuthenticationType::<E>::ResourcePermissionAuthenticator::extract_resource_id
	/// [2]: AuthenticationType::<E>::ResourcePermissionAuthenticator::permission
	ResourcePermissionAuthenticator {
		/// This function is used to extract the resource id from the request.
		/// The resource id is then used to check if the user has the specified
		/// permission on the resource.
		extract_resource_id: fn(&ApiRequest<E>) -> Uuid,
		// /// The permission that the user needs to have on the resource.
		// permission: ResourcePermission,
	},
}

impl<E> AuthenticationType<E>
where
	E: ApiEndpoint,
{
	const IS_AUTH: bool = match E::AUTHENTICATION {
		AuthenticationType::NoAuthentication => false,
		_ => true,
	};
}
