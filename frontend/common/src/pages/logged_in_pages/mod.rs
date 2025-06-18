use crate::prelude::*;

/// The list of deployment pages. This module contains the pages related to
/// managing deployments, such as viewing deployment details, creating new
/// deployments, and managing deployment settings.
mod deployment;
/// The Home page. This is the main page that is displayed when the user is
/// logged in. It is the landing page for the application and provides an
/// overview of the user's projects and activities.
mod home;

pub use self::{deployment::*, home::*};

/// The list of all pages in the application that does requires the user to be
/// logged in
#[derive(Debug, Clone, PartialEq, Eq, Routable)]
pub enum LoggedInRoutes {
	/// The Home page. This is the main page that is displayed when the user
	/// is logged in. It is the landing page for the application and provides
	/// an overview of the user's projects and activities.
	#[route("/", HomePage)]
	Home,
}
