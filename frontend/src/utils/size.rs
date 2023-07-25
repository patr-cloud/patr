#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
	ExtraExtraLarge,
	ExtraLarge,
	Large,
	#[default]
	Medium,
	Small,
	ExtraSmall,
	ExtraExtraSmall,
}

impl Size {
	pub const fn as_css_name(&self) -> &'static str {
		match self {
			ExtraExtraLarge => "xxl",
			ExtraLarge => "xl",
			Large => "lg",
			Medium => "md",
			Small => "sm",
			ExtraSmall => "xs",
			ExtraExtraSmall => "xxs",
		}
	}
}

pub use Size::*;
