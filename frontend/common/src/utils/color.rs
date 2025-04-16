use std::fmt::{self, Display, Formatter};

/// All colors supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
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

impl Display for Color {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl Color {
	/// Returns the CSS name of the color.
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Primary => "primary",
			Self::Secondary => "secondary",
			Self::White => "white",
			Self::Black => "black",
			Self::Grey => "grey",
			Self::Success => "success",
			Self::Warning => "warning",
			Self::Error => "error",
			Self::Info => "info",
			Self::Disabled => "disabled",
		}
	}

	/// Returns the text color corresponding to this color.
	pub const fn as_text_color(self) -> TextColor {
		TextColor(self)
	}
}

/// All text colors supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextColor(pub Color);

impl Display for TextColor {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_color())
	}
}

impl TextColor {
	/// Returns the CSS name of the text color.
	pub const fn as_css_color(self) -> &'static str {
		match self.0 {
			Color::Primary => "text-primary",
			Color::Secondary => "text-secondary",
			Color::White => "text-white",
			Color::Black => "text-black",
			Color::Grey => "text-grey",
			Color::Success => "text-success",
			Color::Warning => "text-warning",
			Color::Error => "text-error",
			Color::Info => "text-info",
			Color::Disabled => "text-disabled",
		}
	}
}
