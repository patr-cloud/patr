/// The Main Container for all the content, typically used alongside the sidebar
/// and used for LoggedIn Routes
mod main_container;
/// A Single Page container, typically will wrap around all pages
mod page_container;

pub use self::{main_container::*, page_container::*};
