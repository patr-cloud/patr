use std::fmt::{self, Display, Formatter};

/// The type of alert to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertType {
	/// Show an error Alert, with a red background
	Error,
	/// Show a warning Alert, with a yellow background
	Warning,
	/// Show a success Alert, with a green background
	Success,
}

impl Display for AlertType {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl AlertType {
	/// Returns the CSS name of the color.
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Error => "error",
			Self::Warning => "warning",
			Self::Success => "success",
		}
	}
}
