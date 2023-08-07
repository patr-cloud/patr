mod color;
mod ext;
mod fetch;
mod notification_type;
mod routes;
mod size;
mod state;
mod variant;

pub use self::{
	color::*,
	ext::*,
	fetch::*,
	notification_type::*,
	routes::*,
	size::*,
	state::*,
	variant::*,
};

/// All the constants used in the application.
/// Constants are used to avoid hardcoding values, since that might introduce
/// typos.
pub mod constants {
	/// Path to the Feather icon sprite.
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";

	/// Base URL for the API
	pub const API_BASE_URL: &str = "https://api.patr.cloud";
}
