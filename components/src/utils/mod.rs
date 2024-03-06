mod alignment;
mod app_route;
mod color;
mod size;
mod variant;

pub use self::{alignment::*, app_route::*, color::*, size::*, variant::*};

/// The constants used within the components
pub mod constants {
	/// The path to the feather icons sprite
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";
}
