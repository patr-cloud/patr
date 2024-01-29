pub mod prelude {
	pub use crate::{containers::*, icon::*, input::*, link::*, sidebar::*, utils::*};
}

mod imports {
	pub use leptos::*;

	pub use crate::prelude::*;
}

pub mod containers;
pub mod icon;
pub mod input;
pub mod link;
pub mod pages;
pub mod sidebar;

pub mod utils;
