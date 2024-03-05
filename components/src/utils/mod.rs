mod alignment;
mod color;
mod size;
mod variant;

pub use self::{alignment::*, color::*, size::*, variant::*};

/// The constants used within the components
pub mod constants {
	/// The path to the feather icons sprite
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";
}
