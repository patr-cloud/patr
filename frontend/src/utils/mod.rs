mod color;
mod ext;
mod fetch;
mod notification_type;
mod size;
mod state;
mod variant;

pub use self::{
	color::*,
	ext::*,
	fetch::*,
	notification_type::*,
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
