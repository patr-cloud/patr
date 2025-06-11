/// The list of all pages in the application that require the user to be logged
/// in
mod logged_in_pages;
/// The list of all pages in the application that does not require the user to
/// be logged in
mod logged_out_pages;
/// The Not Found page
mod not_found;

pub use self::{logged_in_pages::*, logged_out_pages::*, not_found::*};
