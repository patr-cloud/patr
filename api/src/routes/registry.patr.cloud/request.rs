use std::net::IpAddr;

use axum::body::Body;
use models::RequestUserData;
use preprocess::Preprocessable;
use rustis::client::Client as RedisClient;

use crate::{routes::registry_patr_cloud::prelude::*, utils::config::AppConfig};

/// This struct represents a unprocessed request to the API. It contains the
/// path, query, headers and unprocessed body of the request. This struct
/// provides a builder API to make it easier to construct requests.
pub struct RegistryUnprocessedApiRequest<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Parsed path parameters (e.g., repository name, digest, reference)
	pub path: E::RequestPath,
	/// Parsed query parameters
	pub query: E::RequestQuery,
	/// Parsed request headers
	pub headers: E::RequestHeaders,
	/// Streaming request body (not buffered in memory)
	pub body: Body,
}

/// This struct represents a preprocessed request to the API. It contains the
/// path, query, headers and preprocessed body of the request. This struct
/// provides a builder API to make it easier to construct requests.
pub struct RegistryProcessedApiRequest<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Parsed path parameters (e.g., repository name, digest, reference)
	pub path: <E::RequestPath as Preprocessable>::Processed,
	/// Parsed query parameters
	pub query: <E::RequestQuery as Preprocessable>::Processed,
	/// Parsed request headers
	pub headers: E::RequestHeaders,
	/// Streaming request body (not buffered in memory)
	pub body: Body,
}

impl<E> TryFrom<RegistryUnprocessedApiRequest<E>> for RegistryProcessedApiRequest<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Error = preprocess::Error;

	fn try_from(value: RegistryUnprocessedApiRequest<E>) -> Result<Self, Self::Error> {
		let RegistryUnprocessedApiRequest {
			path,
			query,
			headers,
			body,
		} = value;
		Ok(RegistryProcessedApiRequest {
			path: path.preprocess()?,
			query: query.preprocess()?,
			headers,
			body,
		})
	}
}

/// Request object for registry endpoints that do not require authentication.
/// This struct contains all the parsed request data along with streaming body
/// support for efficient handling of large blobs without buffering.
///
/// This struct is similar to [`RegistryAppRequest`] but uses the unprocessed
/// request type. Once the request is validated and preprocessed, it can be
/// converted to [`RegistryAppRequest`].
pub struct RegistryUnprocessedAppRequest<'a, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: RegistryUnprocessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The S3 bucket to put objects and take from.
	pub s3: Box<s3::Bucket>,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The application configuration.
	pub config: AppConfig,
}

/// Request object for registry endpoints that do not require authentication.
///
/// This struct contains all the parsed request data along with streaming body
/// support for efficient handling of large blobs without buffering.
pub struct RegistryAppRequest<'a, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: RegistryProcessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The S3 bucket to put objects and take from.
	pub s3: Box<s3::Bucket>,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The application configuration.
	pub config: AppConfig,
}

/// Request object for registry endpoints that require authentication.
///
/// This struct extends `RegistryRequest` with user authentication data,
/// allowing handlers to verify workspace access and permissions.
pub struct AuthenticatedRegistryAppRequest<'a, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: RegistryProcessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The S3 bucket to put objects and take from.
	pub s3: Box<s3::Bucket>,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The user data of the current authenticated user.
	pub user_data: RequestUserData,
	/// The application configuration.
	pub config: AppConfig,
}
