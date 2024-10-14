/// Middleware to check if the user is authenticated. If not, it will redirect
/// to the login page with the current path as a query parameter.
mod authenticator;

pub use self::authenticator::*;
