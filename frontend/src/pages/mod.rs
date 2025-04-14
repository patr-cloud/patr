/// The Auth Pages, such as Login, Register, and Forgot Password
mod auth;
/// The Deployments set of page, contains, create, list, and update deployments
/// pages
mod deployment;
/// The Home page
mod home;

pub use self::{auth::*, deployment::*, home::*};
