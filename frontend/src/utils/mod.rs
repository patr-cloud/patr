mod app_route;
mod client;
mod color;
mod ext;
mod notification_type;
mod routes;
mod size;
mod state;
mod variant;

pub use self::{
	app_route::*,
	client::*,
	color::*,
	ext::*,
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
}
