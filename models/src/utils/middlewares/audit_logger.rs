use std::fmt::{Debug, Display};

use preprocess::Preprocessable;

use crate::{prelude::*, rbac::ResourceType, utils::RequiresRequestHeaders};

/// This enum represents the different types of actions that can be logged in
/// the audit logs. This is used in the [`AuditLogger`] struct to specify the
/// type of action that is being performed on the endpoint, such as creating a
/// resource, updating a resource, or deleting a resource. This information is
/// used to log the action in the audit logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
	not(target_arch = "wasm32"),
	derive(sqlx::Type),
	sqlx(type_name = "AUDIT_LOG_TYPE", rename_all = "lowercase")
)]
pub enum AuditLogType {
	/// The action being logged is the creation of a resource.
	#[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "create"))]
	ResourceCreated,
	/// The action being logged is the update of a resource.
	#[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "update"))]
	ResourceUpdated,
	/// The action being logged is the deletion of a resource.
	#[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "delete"))]
	ResourceDeleted,
}

impl Display for AuditLogType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ResourceCreated => write!(f, "Created"),
			Self::ResourceUpdated => write!(f, "Updated"),
			Self::ResourceDeleted => write!(f, "Deleted"),
		}
	}
}

/// This enum represents the different ways in which the resource ID can be
/// extracted for logging in the audit logs. This is used in the
/// [`AppAuditLogger`] struct to specify how the ID of the resource that is
/// being created / updated / deleted can be extracted for logging in the audit
/// logs. The resource ID is used to log the action on the specific resource in
/// the audit logs.
#[derive(Clone, Copy)]
pub enum ResourceIdExtractor<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The resource ID is extracted from the request using the provided
	/// function.
	FromRequest(fn(&ProcessedApiRequest<E>) -> Uuid),
	/// The resource ID is extracted from the response using the provided
	/// function.
	FromResponse(fn(&AppResponse<E>) -> Uuid),
}

/// This enum represents the different types of AuditLogger that can be used
/// for an API endpoint.
///
/// The variants are:
/// - [`NoAuditLogger`][1]: This struct is used to specify that an API endpoint
///   does not require auditing. It does not log any actions performed on this
///   endpoint.
/// - [`AppAuditLogger`][2]: This struct is used to specify that an API endpoint
///   requires auditing. It logs all actions performed on this resource,
///   including the request, the response, and the user that performed the
///   action.
///
/// This enum is used in the [`ApiEndpoint`] trait to specify the AuditLogger
/// type of an endpoint. The constant in the trait is used by the router
/// extension to mount the corresponding [`tower::Layer`] in the router.
///
/// [1]: AuditLogger::NoAuditLogger
/// [2]: AuditLogger::AppAuditLogger
#[derive(Clone, Copy)]
pub enum AuditLogger<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// This variant is used to specify that an API endpoint does not require
	/// auditing. It does not log any actions performed on this endpoint.
	NoAuditLogger,
	/// This variant is used to specify that an API endpoint requires auditing.
	/// It logs all actions performed on this resource, including the request,
	/// the response, and the user that performed the action.
	AppAuditLogger {
		/// The type of audit log that is being logged. This is used to specify
		/// the type of action that is being performed on the endpoint, such
		/// as creating a resource, updating a resource, or deleting a
		/// resource. This information is used to log the action in the audit
		/// logs.
		audit_log_type: AuditLogType,
		/// The type of resource that is being logged.
		resource_type: ResourceType,
		/// A function that takes in the processed request and extracts the ID
		/// of the resource that is being created. This is used to log the
		/// creation / update / deletion of the resource in the audit logs.
		extract_resource_id: ResourceIdExtractor<E>,
	},
}

impl<E> RequiresRequestHeaders for AuditLogger<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type RequiredRequestHeaders = (BearerToken,);
}

impl<E> Debug for AuditLogger<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoAuditLogger => write!(f, "NoAuditLogger"),
			Self::AppAuditLogger {
				audit_log_type,
				resource_type,
				extract_resource_id: _,
			} => write!(f, "{} {}", resource_type, audit_log_type),
		}
	}
}
