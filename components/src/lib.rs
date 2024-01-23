pub mod prelude {
	pub use crate::{input::*, link::*, utils::*};
}

mod imports {
	pub use leptos::*;

	pub use crate::prelude::*;
}

pub mod input;
pub mod link;

pub mod utils;
