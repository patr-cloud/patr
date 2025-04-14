/// The Body of the dashboard. Wraps around the main content of the page.
mod container_body;
/// A Grid container, used in the Dashboard to show multiple items in a grid,
/// such as the Deployments Dashboard
mod container_grid;
/// Contains the title, description and the DocLink of the Page,
/// Usually wrapping around the <PageTitle /> section of components
mod container_head;
/// The Main Container for all the content, typically used alongside the sidebar
/// and used for LoggedIn Routes
mod container_main;

pub use self::{container_body::*, container_grid::*, container_head::*, container_main::*};
