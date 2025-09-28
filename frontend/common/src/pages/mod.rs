/// The Auth Pages, such as Login, Register, and Forgot Password
mod auth;
/// The Deployments set of page, contains, create, list, and update deployments
/// pages
mod deployment;
/// The Home page
mod home;
/// The 404 not found page when no other route is matched
mod not_found;

pub use self::{auth::*, deployment::*, home::*, not_found::*};
