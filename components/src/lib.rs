pub mod prelude {
	pub use crate::{
		containers::*,
		dashboard_container::*,
		icon::*,
		input::*,
		link::*,
		page_title::*,
		sidebar::*,
		status_badge::*,
		table_dashboard::*,
		utils::*,
	};
}

mod imports {
	pub use leptos::*;

	pub use crate::prelude::*;
}

pub mod containers;
pub mod dashboard_container;
pub mod icon;
pub mod input;
pub mod link;
pub mod page_title;
pub mod pages;
pub mod sidebar;
pub mod status_badge;
pub mod table_dashboard;

pub mod utils;
