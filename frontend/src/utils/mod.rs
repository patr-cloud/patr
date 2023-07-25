mod fetch;
mod notification_type;
mod patr_color;
mod size;
mod state;
mod variant;

pub use self::{
	fetch::*,
	notification_type::*,
	patr_color::*,
	size::*,
	state::*,
	variant::*,
};

pub mod constants {
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";
}
