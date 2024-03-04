#![warn(missing_docs)]

//! The components module. This module contains all the components that are used
//! across all applications in the Patr ecosystem.

/// The prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use crate::{
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

	pub use crate::prelude::*;
}

pub mod backdrop;
pub mod checkbox_dropdown;
pub mod containers;
pub mod dashboard_container;
pub mod double_input_slider;
pub mod icon;
pub mod input;
pub mod input_dropdown;
pub mod link;
pub mod log_statement;
pub mod number_picker;
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
