#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecondaryColorVariant {
	#[default]
	Light,
	Medium,
	Dark,
}

impl SecondaryColorVariant {
	pub const fn as_css_name(&self) -> &'static str {
		match self {
			Light => "light",
			Medium => "medium",
			Dark => "dark",
		}
	}
}

pub use SecondaryColorVariant::*;
