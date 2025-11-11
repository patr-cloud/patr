use std::{
	collections::BTreeMap,
	future::Future,
	marker::PhantomData,
	str::FromStr as _,
	task::{Context, Poll},
};

use models::utils::{AppAuthentication, BearerToken, HasHeader};
use preprocess::Preprocessable;
use tokio::sync::OnceCell;
use tower::{Layer, Service};

use crate::prelude::*;

static PERMISSION_TO_ID_MAP: OnceCell<BTreeMap<Permission, Uuid>> = OnceCell::const_new();

/// The [`tower::Layer`] used to authorize requests. This will check the
/// permissions of the authenticated user against the required permissions for
/// the endpoint. If the user has the required permissions, the request will be
/// passed to the next layer. Otherwise, an error will be returned.
pub struct AuthorizationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<E> AuthorizationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize an authorization layer
	pub fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for AuthorizationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AuthenticatedAppRequest<'a, E>>,
{
	type Service = AuthorizationService<E::Authenticator, E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthorizationService {
			inner,
			authenticator: PhantomData,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuthorizationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service that runs when the [`AuthorizationLayer`] is used.
pub struct AuthorizationService<A, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request is authenticated
	inner: S,
	/// The type of authenticator that will be used to authenticate the request
	authenticator: PhantomData<A>,
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AuthenticatedAppRequest<'a, E>>
	for AuthorizationService<AppAuthentication<E>, E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestHeaders: HasHeader<BearerToken>,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "AuthenticatorService")]
	fn call(&mut self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Authorizing request");

			let auth = E::get_authenticator();
			let authorized = match auth {
				AppAuthentication::PlainTokenAuthenticator => true,
				AppAuthentication::WorkspaceMembershipAuthenticator {
					extract_workspace_id,
				} => {
					let workspace_id = extract_workspace_id(&req.request);
					req.user_data.permissions.contains_key(&workspace_id)
				}
				AppAuthentication::WorkspaceSuperAdminAuthenticator {
					extract_workspace_id,
				} => {
					let workspace_id = extract_workspace_id(&req.request);
					req.user_data
						.permissions
						.get(&workspace_id)
						.map_or(false, |perms| perms.is_super_admin())
				}
				AppAuthentication::ResourcePermissionAuthenticator {
					extract_resource_id,
					extract_workspace_id,
					permission,
				} => {
					let workspace_id = extract_workspace_id(&req.request);
					let resource_id = extract_resource_id(&req.request);

					let permission_id = PERMISSION_TO_ID_MAP
						.get_or_init(async || {
							query!(
								r#"
								SELECT
									id AS "id: Uuid",
									name
								FROM
									permission;
								"#
							)
							.fetch_all(&mut **req.database)
							.await
							.unwrap_or_default()
							.into_iter()
							.map(|row| {
								(
									Permission::from_str(&row.name)
										.expect("Invalid permission name"),
									row.id,
								)
							})
							.collect()
						})
						.await
						.get(&permission)
						.copied()
						.ok_or_else(|| {
							error!("Permission {permission} does not exist in the database");
							ErrorType::InternalServerError
						})?;

					// Check if the user has the required permission for the resource in the
					// workspace
					let has_permission = req
						.user_data
						.permissions
						.get(&workspace_id)
						.map_or(false, |perms| {
							perms.has_resource_permission(resource_id, permission_id)
						});

					let has_permission = if !has_permission {
						warn!(
							"User {} does not have permission {} for resource {} in workspace {}",
							req.user_data.id, permission, resource_id, workspace_id
						);
						false
					} else {
						true
					};

					// Additionally, check that the resource actually exists in the (right)
					// workspace.
					let resource = query!(
						r#"
						SELECT
							id,
							owner_id
						FROM
							resource
						WHERE
							id = $1;
						"#,
						resource_id as _
					)
					.fetch_optional(&mut **req.database)
					.await?;

					let exists = if let Some(resource) = resource {
						if resource.owner_id == workspace_id {
							true
						} else {
							warn!(
								"Resource {} exists, but does not belong to workspace {}",
								resource_id, workspace_id
							);
							debug!("It actually belongs to workspace {}", resource.owner_id);
							false
						}
					} else {
						warn!("Resource {} does not exist", resource_id);
						false
					};

					// The user must both have the permission and the resource must exist in that
					// workspace
					has_permission && exists
				}
			};

			if !authorized {
				trace!("Authorization failed");
				return Err(ErrorType::Unauthorized);
			}

			inner.call(req).await
		}
	}
}

impl<A, E, S> Clone for AuthorizationService<A, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			authenticator: PhantomData,
			endpoint: PhantomData,
		}
	}
}
