#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! The components module. This module contains all the components that are used
//! across all applications in the Patr ecosystem.

/// The prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use crate::{
		alert::*,
		backdrop::*,
		checkbox_dropdown::*,
		containers::*,
		dashboard_container::*,
		double_input_slider::*,
		icon::*,
		input::*,
		input_dropdown::*,
		link::*,
		log_statement::*,
		modal::*,
		number_picker::*,
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
	pub use leptos_router::A;

	pub use crate::prelude::*;
}

/// The Alert component. The alert component is used to display an alert to the
/// user. It is used to show the user a message, like a success message, a
/// warning message, or an error message. The alert disappeares after a few
/// seconds
pub mod alert;
/// The backdrop component. The backdrop component is used to display a backdrop
/// that can be used to block the user from interacting with the rest of the
/// page. It is used to show the user that something is loading or in modals
pub mod backdrop;
/// The Checkbox Dropdown component. The checkbox dropdown component is used to
/// display a dropdown with checkboxes.
pub mod checkbox_dropdown;
/// The containers module. This module contains all the container components. A
/// container is a component that can be used to hold other components (like a
/// box).
pub mod containers;
/// The dashboard container module. This module contains the dashboard container
/// component. The dashboard container is a container that is used to hold
/// dashboard components.
pub mod dashboard_container;
/// The Double Input Slider component. The Double Input Slider component is used
/// to display a slider with two inputs. It is used to allow the user to select
/// two values from a range of values.
pub mod double_input_slider;
/// The Icon component. This component is used to display an icon in the
/// fontawsome library, of different sizes and colors.
pub mod icon;
/// The input component. The input component is used for text, email, and other
/// things.
pub mod input;
/// The Input Dropdown component. The input dropdown component is used to
/// display a dropdown input. It is used to allow the user to select an option
/// from a list of options.
pub mod input_dropdown;
/// The link component. The link component is used to create a link to another
/// page, or to an external website. It can also be used to create a button.
pub mod link;
/// The Log Statement component. The log statement component is used to display
/// a log statement. It is used to show the user a log statement, like a
/// deployment log, or a database log in the Deployments page
pub mod log_statement;
/// The Modal component. The modal component is used to display a modal. It is
/// used to show the user a modal, like a confirmation modal, or a settings
/// modal.
pub mod modal;
/// The number picker component. The number picker component is used to display
/// a number picker. It is used to allow the user to select a number from a
/// range of numbers.
pub mod number_picker;
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
/// The Textbox component. The textbox component is used to display a textbox
/// input when the user cannot edit the value.
pub mod textbox;
/// The Tooltip component. The tooltip component is used to display a tooltip
/// when the user hovers over an element. It is used to show the user more
/// information about an element.
pub mod tooltip;

/// Utility functions and components that are used across the entire crate.
pub mod utils;
