/// A struct representing the audit logging to be stored when an endpoint is
/// hit. This will be used by the API when an endpoint is hit to store the audit
/// log. This way, all endpoints will necessarily have an audit log attached to
/// it to ensure that nothing is missed, even by mistake.
mod audit_logger;
/// A middleware to represent the kind of authentication required for an API
/// endpoint. More details can be found in the documentation for the
/// [`AppAuthentication`] enum.
mod authentication_type;

#[allow(unused_imports)]
pub use self::{audit_logger::*, authentication_type::*};
