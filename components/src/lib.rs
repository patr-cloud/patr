// #![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! The components module. This module contains all the components that are used
//! across all applications in the Patr ecosystem.

/// The prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use crate::{
		backdrop::*,
		containers::*,
		dashboard_container::*,
		icon::*,
		input::*,
		link::*,
		otp_input::*,
		page_title::*,
		sidebar::*,
		skeleton::*,
		spinner::*,
		status_badge::*,
		table_dashboard::*,
		textbox::*,
		tooltip::*,
		utils::*,
	};
}

/// The imports module. This is basically similar to a prelude, but for within
/// the crate
mod imports {
	use std::rc::Rc;

	/// The handler for the click event on a component. This can be either a
	/// function or a closure that takes a MouseEvent as an argument.
	pub(crate) type ClickHandler = Rc<dyn Fn(&ev::MouseEvent)>;

	pub use leptos::*;

	pub use crate::prelude::*;
}

/// The backdrop component. The backdrop component is used to display a backdrop
/// that can be used to block the user from interacting with the rest of the
/// page. It is used to show the user that something is loading or in modals
pub mod backdrop;
/// The containers module. This module contains all the container components. A
/// container is a component that can be used to hold other components (like a
/// box).
pub mod containers;
/// The dashboard container module. This module contains the dashboard container
/// component. The dashboard container is a container that is used to hold
/// dashboard components.
pub mod dashboard_container;
/// The Icon component. This component is used to display an icon in the
/// fontawsome library, of different sizes and colors.
pub mod icon;
/// The input component. The input component is used for text, email, and other
/// things.
pub mod input;
/// The link component. The link component is used to create a link to another
/// page, or to an external website. It can also be used to create a button.
pub mod link;
/// The OTP input component. The OTP input component is used to display an input
/// for an OTP code. It is used in the login page.
pub mod otp_input;
/// The page title component. The page title component is used to display the
/// title of a page, specifically the title of a dashboard page.
pub mod page_title;
/// The sidebar component. The sidebar component is used to display a sidebar
/// that can be used to navigate between different pages. It is used in the
/// dashboard, and will reactively change the page when a link is clicked or
/// when a new page is loaded.
pub mod sidebar;
/// The skeleton component. The skeleton component is used to display a loading
/// skeleton for a component that is loading. It is used to show the user that
/// something is loading, and that they should wait. This is not needed for
/// server side rendered components, but doesn't hurt to have since it will be
/// replaced by the actual component when it loads.
pub mod skeleton;
/// The spinner component. The spinner component is used to display a loading
/// spinner. It is used to show the user that something is loading, and that
/// they should wait. This is not needed for situations when javascript / WASM
/// hasn't loaded yet, but can be kept since those situations do a full page
/// reload anyway.
pub mod spinner;
/// The status badge component. The status badge component is used to display a
/// status badge, like a success badge, a warning badge, or an error badge. It
/// is used to show the user the status of something, like a database, or a
/// deployment.
pub mod status_badge;
/// The table dashboard component. The table dashboard component is used to
/// display a table of data in a dashboard. It is used to show the user a table
/// of data, like a list of users, or a list of deployments, etc.
pub mod table_dashboard;
pub mod textbox;
pub mod tooltip;

/// Utility functions and components that are used across the entire crate.
pub mod utils;
