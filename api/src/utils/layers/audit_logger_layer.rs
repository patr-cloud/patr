use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use headers::UserAgent;
use models::utils::{AppAuthentication, AuditLogger, HasHeader, ResourceIdExtractor};
use preprocess::Preprocessable;
use sqlx::types::ipnetwork::IpNetwork;
use tower::{Layer, Service};

use crate::prelude::*;

/// The [`tower::Layer`] used to handle audit logging of requests. This layer is
/// responsible for logging the actions performed on the endpoint, including the
/// request, the response, and the user that performed the action. This layer is
/// only used for endpoints that require auditing, as specified by the
/// [`AppAuditLogger`] struct in the [`ApiEndpoint`] trait.
pub struct AuditLoggerLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<E> Default for AuditLoggerLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> AuditLoggerLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize a new preprocess layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for AuditLoggerLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AuthenticatedAppRequest<'a, E>>,
{
	type Service = AuditLoggerService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuditLoggerService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuditLoggerLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service that runs when the [`AuditLoggerLayer`] is used.
pub struct AuditLoggerService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request's login Id is
	/// handled
	inner: S,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AuthenticatedAppRequest<'a, E>> for AuditLoggerService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestHeaders: HasHeader<UserAgent>,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "AuditLoggerService")]
	fn call(&mut self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Preprocessing request");

			let AuthenticatedAppRequest {
				request,
				database,
				redis,
				client_ip,
				user_data,
				state,
			} = req;

			let (audit_log_type, _, extract_resource_id) = match E::get_audit_logger() {
				AuditLogger::AppAuditLogger {
					audit_log_type,
					resource_type,
					extract_resource_id,
				} => (audit_log_type, resource_type, extract_resource_id),
				AuditLogger::NoAuditLogger => {
					return inner
						.call(AuthenticatedAppRequest {
							request,
							database,
							redis,
							client_ip,
							user_data,
							state,
						})
						.await;
				}
			};

			let user_agent = request.headers.get_header().as_str().to_owned();
			let login_id = user_data.login_id;
			let ip_details = ip::lookup(client_ip, redis, &state.config.ipinfo).await?;
			let client_ip_network = IpNetwork::from(client_ip);

			let (lat, lng) = ip_details
				.loc
				.split_once(',')
				.map(|(lat, lng)| (lat.trim(), lng.trim()))
				.and_then(|(lat, lng)| {
					lat.parse::<f64>()
						.inspect_err(|err| {
							info!("Error parsing latitude: `{lat}` - {err}");
						})
						.ok()
						.zip(
							lng.parse::<f64>()
								.inspect_err(|err| {
									info!("Error parsing longitude: `{lng}` - {err}");
								})
								.ok(),
						)
				})
				.unwrap_or((0f64, 0f64));

			let country = ip_details.country;
			let region = ip_details.region;
			let city = ip_details.city;
			let timezone = ip_details.timezone.unwrap_or_default();

			let (resource_id, response) = match extract_resource_id {
				ResourceIdExtractor::FromRequest(extractor) => {
					let resource_id = extractor(&request);
					(
						resource_id,
						inner
							.call(AuthenticatedAppRequest {
								request,
								database,
								redis,
								client_ip,
								user_data,
								state,
							})
							.await?,
					)
				}
				ResourceIdExtractor::FromResponse(extractor) => {
					let response = inner
						.call(AuthenticatedAppRequest {
							request,
							database,
							redis,
							client_ip,
							user_data,
							state,
						})
						.await?;
					let resource_id = extractor(&response);
					(resource_id, response)
				}
			};

			query!(
				r#"
				INSERT INTO
					audit_log(
						id,

						timestamp,
						ip,
						location,
						user_agent,
						country,
						region,
						city,
						timezone,

						login_id,
						action,
						workspace_id,
						resource_id
					)
				VALUES
					(
						GENERATE_AUDIT_LOG_ID(),
						
						NOW(),
						$1,
						ST_SetSRID(POINT($2, $3)::GEOMETRY, 4326),
						$4,
						$5,
						$6,
						$7,
						$8,

						$9,
						$10,
						(SELECT owner_id FROM resource WHERE id = $11),
						$11
					);
				"#,
				client_ip_network,
				lat,
				lng,
				user_agent,
				country,
				region,
				city,
				timezone,
				login_id as _,
				audit_log_type as _,
				resource_id as _,
			)
			.execute(&mut **database)
			.await?;

			// TODO audit log changes

			Ok(response)
		}
	}
}

impl<E, S> Clone for AuditLoggerService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
