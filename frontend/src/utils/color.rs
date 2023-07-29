use std::fmt::{self, Display, Formatter};

/// All colors supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatrColor {
	/// Primary yellow color. This is the default.
	#[default]
	Primary,
	/// Secondary purple color.
	Secondary,
	/// White color.
	White,
	/// Black color.
	Black,
	/// Grey color.
	Grey,
	/// Success green color.
	Success,
	/// Warning orange color.
	Warning,
	/// Error red color.
	Error,
	/// Info blue color.
	Info,
	/// Disabled color.
	Disabled,
}

impl Display for PatrColor {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl PatrColor {
	/// Returns the CSS name of the color.
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Primary => "primary",
			Secondary => "secondary",
			White => "white",
			Black => "black",
			Grey => "grey",
			Success => "success",
			Warning => "warning",
			Error => "error",
			Info => "info",
			Disabled => "disabled",
		}
	}

	/// Returns the text color corresponding to this color.
	pub const fn as_text_color(self) -> TextColor {
		TextColor(self)
	}
}

pub use PatrColor::*;

/// All text colors supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextColor(pub PatrColor);

impl Display for TextColor {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_color())
	}
}

impl TextColor {
	/// Returns the CSS name of the text color.
	pub const fn as_css_color(self) -> &'static str {
		match self.0 {
			Primary => "txt-primary",
			Secondary => "txt-secondary",
			White => "txt-white",
			Black => "txt-black",
			Grey => "txt-grey",
			Success => "txt-success",
			Warning => "txt-warning",
			Error => "txt-error",
			Info => "txt-info",
			Disabled => "txt-disabled",
		}
	}
}
