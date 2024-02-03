#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
	Left,
	Right,
	#[default]
	Center,
}

impl Alignment {
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Center => "center",
			Self::Left => "left",
			Self::Right => "right",
		}
	}
}
