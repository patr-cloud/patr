use std::fmt::{self, Display, Formatter};

/// The Color variants supported by the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecondaryColorVariant {
	/// Default. Light Color Variant
	#[default]
	Light,
	/// Medium Color variant
	Medium,
	/// Dark Color variant
	Dark,
}

impl Display for SecondaryColorVariant {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl SecondaryColorVariant {
	/// Returns the css class name correspoding to the variant
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Light => "light",
			Self::Medium => "medium",
			Self::Dark => "dark",
		}
	}
}

/// Link Variant
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Variant {
	/// A Normal Button. To be used with the Link Component
	#[default]
	Button,
	/// A Link. To be used with the Link Component
	Link,
}

/// The Type of Link to use. A contained link is a button with a background,
/// while a plain link looks like an anchor tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LinkStyleVariant {
	/// An Outlined Link. This is a button without a background, but with an
	/// outline.
	Outlined,
	/// A contained link. This is a button with a background.
	Contained,
	/// A plain link. This looks like an anchor tag.
	#[default]
	Plain,
}
