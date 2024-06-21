use std::fmt::Debug;

use preprocess::Preprocessable;

use crate::{prelude::*, utils::RequiresRequestHeaders};

/// This struct is used to specify that an API endpoint does not require
/// authentication. It can be accessed without any token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoAuthentication;

impl RequiresRequestHeaders for NoAuthentication {
	type RequiredRequestHeaders = ();
}

/// This enum represents the different types of authentication that can be used
/// for an API endpoint.
///
/// The variants are:
/// - [`PlainTokenAuthenticator`][1]: Any logged in user can access this
///   endpoint.
/// - [`WorkspaceSuperAdminAuthenticator`][2]: Only the super admin of the
///   workspace that is specified in the request can access this endpoint.
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
/// [1]: AppAuthentication::PlainTokenAuthenticator
/// [2]: AppAuthentication::WorkspaceSuperAdminAuthenticator
/// [3]: AppAuthentication::WorkspaceMembershipAuthenticator
/// [4]: AppAuthentication::ResourcePermissionAuthenticator
pub enum AppAuthentication<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Any logged in user can access this endpoint.
	PlainTokenAuthenticator,
	/// Only the super admin of the workspace that is specified in the
	/// [`extract_workspace_id`][1] function can access this endpoint.
	///
	/// [1]: AppAuthentication::<E>::WorkspaceSuperAdminAuthenticator::extract_workspace_id
	WorkspaceSuperAdminAuthenticator {
		/// This function is used to extract the workspace id from the request.
		/// The workspace id is then used to check if the user is the super
		/// admin of the workspace.
		extract_workspace_id: fn(&ApiRequest<E>) -> Uuid,
	},
	/// Only users that are members of the workspace that is specified in the
	/// [`extract_workspace_id`][1] function can access this endpoint.
	///
	/// [1]: AppAuthentication::<E>::WorkspaceMembershipAuthenticator::extract_workspace_id
	WorkspaceMembershipAuthenticator {
		/// This function is used to extract the workspace id from the request.
		/// The workspace id is then used to check if the user is a member of
		/// the workspace.
		extract_workspace_id: fn(&ApiRequest<E>) -> Uuid,
	},
	/// Only users that have the permission specific by [`permission`][2] on the
	/// resource that is specified in the [`extract_resource_id`][1] function
	/// can access this endpoint.
	///
	/// [1]: AppAuthentication::<E>::ResourcePermissionAuthenticator::extract_resource_id
	/// [2]: AppAuthentication::<E>::ResourcePermissionAuthenticator::permission
	ResourcePermissionAuthenticator {
		/// This function is used to extract the resource id from the request.
		/// The resource id is then used to check if the user has the specified
		/// permission on the resource.
		extract_resource_id: fn(&ApiRequest<E>) -> Uuid,
		/// The permission that the user needs to have on the resource.
		permission: Permission,
	},
}

impl<E> Clone for AppAuthentication<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		*self
	}
}

impl<E> Copy for AppAuthentication<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
}

impl<E> RequiresRequestHeaders for AppAuthentication<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type RequiredRequestHeaders = (BearerToken,);
}

impl<E> Debug for AppAuthentication<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::PlainTokenAuthenticator => write!(f, "PlainTokenAuthenticator"),
			Self::WorkspaceSuperAdminAuthenticator { .. } => {
				write!(f, "WorkspaceSuperAdminAuthenticator")
			}
			Self::WorkspaceMembershipAuthenticator { .. } => {
				write!(f, "WorkspaceMembershipAuthenticator")
			}
			Self::ResourcePermissionAuthenticator { .. } => {
				write!(f, "ResourcePermissionAuthenticator")
			}
		}
	}
}
