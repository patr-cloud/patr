/// All auth related frontend routes
pub mod auth;

/// Contains the trait that is used to represent all the data that will be used
/// to route to a URL in the frontend. This trait is implemented for all the
/// URL routes in the frontend. Since it is a large trait, a helper macro is
/// provided to generate a request. See: [`macros::declare_app_route`]
mod typed_route;

pub use self::typed_route::*;
