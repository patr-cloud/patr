/// The Auth Pages, such as Login, Register, and Forgot Password
mod auth;
/// The 404 not found page when no other route is matched
mod not_found;
/// The workspace related pages
mod workspace;

pub use self::{auth::*, not_found::*, workspace::*};
