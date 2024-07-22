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

	const fn as_txt(self) -> &'static str {
		match self {
			Self::Primary => "text-primary",
			Self::Secondary => "text-secondary",
			Self::White => "text-white",
			Self::Black => "text-black",
			Self::Grey => "text-grey",
			Self::Success => "text-success",
			Self::Warning => "text-warning",
			Self::Error => "text-error",
			Self::Info => "text-info",
			Self::Disabled => "text-disabled",
		}
	}

	const fn as_bg(self) -> &'static str {
		match self {
			Self::Primary => "bg-primary",
			Self::Secondary => "bg-secondary",
			Self::White => "bg-white",
			Self::Black => "bg-black",
			Self::Grey => "bg-grey",
			Self::Success => "bg-success",
			Self::Warning => "bg-warning",
			Self::Error => "bg-error",
			Self::Info => "bg-info",
			Self::Disabled => "bg-disabled",
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
			Color::Primary => "txt-primary",
			Color::Secondary => "txt-secondary",
			Color::White => "txt-white",
			Color::Black => "txt-black",
			Color::Grey => "txt-grey",
			Color::Success => "txt-success",
			Color::Warning => "txt-warning",
			Color::Error => "txt-error",
			Color::Info => "txt-info",
			Color::Disabled => "txt-disabled",
		}
	}
}
