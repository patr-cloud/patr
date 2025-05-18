use std::fmt::{self, Display, Formatter};

/// The Color variants supported by the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecondaryColorVariant {
	/// Default. Light Color Variant
	#[default]
	Light,
	/// Medium Color variant
	Medium,
	/// Dark Color variant
	Dark,
}

impl Display for SecondaryColorVariant {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl SecondaryColorVariant {
	/// Returns the css class name corresponding to the variant
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Light => "light",
			Self::Medium => "medium",
			Self::Dark => "dark",
		}
	}
}

/// Link Variant
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Variant {
	/// A Normal Button. To be used with the Link Component
	#[default]
	Button,
	/// A Link. To be used with the Link Component
	Link,
}

/// Button Type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, strum::Display)]
pub enum ButtonType {
	/// A Normal Button.
	#[default]
	Button,
	/// Reset All From Values
	Reset,
	/// Submit Form
	Submit,
}

/// The Type of Link to use. A contained link is a button with a background,
/// while a plain link looks like an anchor tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LinkStyleVariant {
	/// An Outlined Link. This is a button without a background, but with an
	/// outline.
	Outlined,
	/// A contained link. This is a button with a background.
	Contained,
	/// A plain link. This looks like an anchor tag.
	#[default]
	Plain,
}

/// The Link Target Types [MDN Doc](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/a#target)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, strum::Display)]
pub enum LinkTarget {
	/// The current browsing context. (Default)
	#[default]
	#[strum(to_string = "_self")]
	_Self,
	/// Usually a new tab, but users can configure browsers to open a new window
	/// instead.
	#[strum(to_string = "_blank")]
	Blank,
	/// The parent browsing context of the current one. If no parent, behaves as
	/// _self.
	#[strum(to_string = "_parent")]
	Parent,
	/// The topmost browsing context. To be specific, this means the "highest"
	/// context that's an ancestor of the current one. If no ancestors, behaves
	/// as _self.
	#[strum(to_string = "_top")]
	Top,
	/// Allows embedded fenced frames to navigate the top-level frame (i.e.,
	/// traversing beyond the root of the fenced frame, unlike other reserved
	/// destinations). Note that the navigation will still succeed if this is
	/// used outside of a fenced frame context, but it will not act like a
	/// reserved keyword.
	#[strum(to_string = "_unfencedTop")]
	UnfencedTop,
}

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
