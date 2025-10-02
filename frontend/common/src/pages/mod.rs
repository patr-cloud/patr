/// The content to show when the user is logged in
mod logged_in_content;
/// The content to show when the user is not logged in
mod logged_out_content;
/// The 404 not found page when no other route is matched
mod not_found;

pub use self::{logged_in_content::*, logged_out_content::*, not_found::*};
