use std::fmt::{self, Display, Formatter};

/// All sizes supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
	/// XXL
	ExtraExtraLarge,
	/// XL
	ExtraLarge,
	/// LG
	Large,
	/// MD - This is the default.
	#[default]
	Medium,
	/// SM
	Small,
	/// XS
	ExtraSmall,
	/// XXS
	ExtraExtraSmall,
}

impl Display for Size {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl Size {
	/// Returns the CSS name of the size.
	pub const fn as_css_name(&self) -> &'static str {
		match self {
			Self::ExtraExtraLarge => "xxl",
			Self::ExtraLarge => "xl",
			Self::Large => "lg",
			Self::Medium => "md",
			Self::Small => "sm",
			Self::ExtraSmall => "xs",
			Self::ExtraExtraSmall => "xxs",
		}
	}
}