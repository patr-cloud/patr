/// Extensions traits for the `Either` type.
mod either_ext;
/// Contains the extension traits that will be used to add exit signal handling
/// to futures.
mod exitable_ext;
/// Contains the extension traits that will be used with the axum [`Router`][1]
/// to mount the various endpoints on the router.
///
/// [1]: axum::Router
mod router_ext;
/// Contains the extension traits that will be used to timeout futures as
/// they're executing.
mod timeout_ext;
/// Contains the extension traits that will be used to add functionality to the
/// worker, such as sending an email, etc.
mod worker_ext;

pub use self::{either_ext::*, exitable_ext::*, router_ext::*, timeout_ext::*, worker_ext::*};
