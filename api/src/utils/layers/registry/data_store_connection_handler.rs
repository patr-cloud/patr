//! Registry data store connection layer.
//!
//! This layer creates database transactions and Redis connections for registry
//! requests. It takes a `ParsedRegistryRequest` from the request parser layer
//! and adds database and Redis connections, then passes it to the next layer.
//!
//! The database transaction is automatically committed on success or rolled
//! back on error.

use std::{
	future::Future,
	marker::PhantomData,
	net::IpAddr,
	task::{Context, Poll},
};

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Builder as S3Builder};
use oci_spec::distribution::ErrorCode;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::routes::registry_patr_cloud::prelude::*;

/// Layer that creates database transactions and Redis connections for registry
/// requests.
///
/// This layer:
/// 1. Begins a database transaction
/// 2. Clones the Redis client
/// 3. Extracts the client IP address
/// 4. Creates a `RegistryRequestWithConnections` object
/// 5. Calls the inner service
/// 6. Commits the transaction on success or rolls back on error
#[derive(Clone)]
pub struct RegistryDataStoreConnectionLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Application state containing database and Redis pools
	state: AppState,
	/// Phantom data for endpoint type
	phantom: PhantomData<E>,
}

impl<E> RegistryDataStoreConnectionLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Create a new data store connection layer with the given state.
	pub fn with_state(state: AppState) -> Self {
		Self {
			state,
			phantom: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RegistryDataStoreConnectionLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Service = RegistryDataStoreConnectionService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryDataStoreConnectionService {
			inner,
			state: self.state.clone(),
			phantom: PhantomData,
		}
	}
}

/// Tower service that creates database and Redis connections for registry
/// requests.
///
/// This service is created by `RegistryDataStoreConnectionLayer` and handles
/// the transaction lifecycle management.
#[derive(Clone)]
pub struct RegistryDataStoreConnectionService<S, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	inner: S,
	state: AppState,
	phantom: PhantomData<E>,
}

impl<S, E> Service<(RegistryUnprocessedApiRequest<E>, IpAddr)>
	for RegistryDataStoreConnectionService<S, E>
where
	for<'a> S: Service<
			RegistryUnprocessedAppRequest<'a, E>,
			Response = RegistryResponse<E>,
			Error = RegistryError,
		> + Clone,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Error = RegistryError;
	type Response = RegistryResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, request), name = "RegistryDataStoreConnectionService")]
	fn call(
		&mut self,
		(request, client_ip): (RegistryUnprocessedApiRequest<E>, IpAddr),
	) -> Self::Future {
		let mut state = self.state.clone();
		let mut inner = self.inner.clone();

		async move {
			trace!("Creating database transaction and Redis connection");

			// Begin a database transaction
			let mut database = match state.database.begin().await {
				Ok(tx) => {
					debug!("Database transaction created successfully");
					tx
				}
				Err(err) => {
					error!("Failed to begin database transaction: {}", err);
					return RegistryError::server_error(
						ErrorCode::Unsupported,
						if cfg!(debug_assertions) {
							"Internal server error: unable to begin database transaction"
						} else {
							"Internal server error"
						},
					)
					.into_result();
				}
			};

			// Get Redis client
			let redis = &mut state.redis;

			let s3 = S3Builder::new()
				.region(Region::new(state.config.s3.region.clone()))
				.endpoint_url(state.config.s3.endpoint.clone())
				.credentials_provider(
					Credentials::builder()
						.access_key_id(&state.config.s3.key)
						.secret_access_key(&state.config.s3.secret)
						.provider_name("Static")
						.build(),
				)
				.force_path_style(state.config.s3.force_path_style)
				.build();
			let s3 = S3Client::from_conf(s3);

			let config = state.config.clone();

			// Create the request with connections
			let request = RegistryUnprocessedAppRequest {
				request,
				database: &mut database,
				redis,
				s3,
				client_ip,
				config,
			};

			info!("Calling inner service with database and Redis connections");

			// Call the inner service
			match inner.call(request).await {
				Ok(response) => {
					info!("Inner service completed successfully, committing transaction");
					// Commit the transaction on success
					if let Err(err) = database.commit().await {
						error!("Failed to commit database transaction: {}", err);
						return RegistryError::server_error(
							ErrorCode::Unsupported,
							"Internal server error: unable to commit database transaction",
						)
						.into_result();
					}
					debug!("Database transaction committed successfully");
					Ok(response)
				}
				Err(error) => {
					warn!("Inner service failed, rolling back transaction: {}", error);
					// Rollback the transaction on error
					if let Err(err) = database.rollback().await {
						error!("Failed to rollback database transaction: {}", err);
						// Return the original error, not the rollback error
					} else {
						debug!("Database transaction rolled back successfully");
					}
					Err(error)
				}
			}
		}
	}
}
