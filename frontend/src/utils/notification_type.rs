use super::Color;

/// All notification types supported by CSS class names in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationType {
	/// Success notification
	Success,
	/// Warning notification
	Warning,
	/// Error notification
	Error,
}

impl NotificationType {
	/// Returns the CSS name of the notification.
	pub const fn as_css_name(&self) -> &'static str {
		match self {
			Self::Success => "success",
			Self::Warning => "warning",
			Self::Error => "error",
		}
	}

	/// Returns the color of the notification.
	pub const fn as_patr_color(&self) -> Color {
		match self {
			Self::Success => Color::Success,
			Self::Warning => Color::Warning,
			Self::Error => Color::Error,
		}
	}
}

/// Converts a notification type into a color.
impl From<NotificationType> for Color {
	fn from(val: NotificationType) -> Self {
		val.as_patr_color()
	}
}
