#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatrColor {
	#[default]
	Primary,
	Secondary,
	White,
	Black,
	Grey,
	Success,
	Warning,
	Error,
	Info,
	Disabled,
}

impl PatrColor {
	pub const fn as_css_name(&self) -> &'static str {
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

	pub const fn as_css_text_color(&self) -> &'static str {
		match self {
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

pub use PatrColor::*;
