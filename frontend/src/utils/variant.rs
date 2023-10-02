use std::fmt::{self, Display, Formatter};

/// The variants of the secondary color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecondaryColorVariant {
	/// The light variant. This is the default.
	#[default]
	Light,
	/// The medium variant.
	Medium,
	/// The dark variant.
	Dark,
}

impl Display for SecondaryColorVariant {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl SecondaryColorVariant {
	/// Returns the CSS name of the variant.
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Light => "light",
			Self::Medium => "medium",
			Self::Dark => "dark",
		}
	}
}
