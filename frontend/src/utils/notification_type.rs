use super::PatrColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationType {
	Success,
	Warning,
	Error,
}

impl NotificationType {
	pub const fn as_css_name(&self) -> &'static str {
		match self {
			Success => "success",
			Warning => "warning",
			Error => "error",
		}
	}

	pub const fn as_patr_color(&self) -> PatrColor {
		match self {
			Success => PatrColor::Success,
			Warning => PatrColor::Warning,
			Error => PatrColor::Error,
		}
	}
}

impl Into<PatrColor> for NotificationType {
	fn into(self) -> PatrColor {
		self.as_patr_color()
	}
}

pub use NotificationType::*;
