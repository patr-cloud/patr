#![warn(missing_docs)]

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

pub mod backdrop;
pub mod containers;
pub mod dashboard_container;
pub mod icon;
pub mod input;
pub mod link;
pub mod otp_input;
pub mod page_title;
pub mod sidebar;
pub mod skeleton;
pub mod spinner;
pub mod status_badge;
pub mod table_dashboard;
pub mod textbox;
pub mod tooltip;

pub mod utils;
