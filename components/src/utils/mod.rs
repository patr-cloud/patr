/// The alignment enum. This enum is used to specify the alignment of a
/// component of left, right, or center.
mod alignment;
/// The color enum. This enum is used to specify the color of a component. These
/// include the primary and secondary colors of the app.
mod color;
/// The size enum. This enum is used to specify the size of a component. We
/// currently have:
/// - ExtraExtraLarge
/// - ExtraLarge
/// - Large
/// - Medium
/// - Small
/// - ExtraSmall
/// - ExtraExtraSmall
mod size;
/// The variant enum. This enum is used to specify the variant of a component
/// and the color variant.
mod variant;

pub use self::{alignment::*, color::*, size::*, variant::*};

/// The constants used within the components
pub mod constants {
	/// The path to the feather icons sprite
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";
}
