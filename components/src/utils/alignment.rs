/// The alignment of an element.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
	/// Left alignment.
	Left,
	/// Right alignment.
	Right,
	/// Center alignment.
	#[default]
	Center,
}

impl Alignment {
	/// Converts the alignment to css class name
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Center => "center",
			Self::Left => "left",
			Self::Right => "right",
		}
	}
}
