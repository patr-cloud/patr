use crate::prelude::*;

/// The Button Component, similar to the HTML Button, just with a few extra
/// props to match patr's theme
#[component]
pub fn Button(
	/// Button Variant i.e. a button or a Link,
	/// Defaults to Button
	#[props(into, optional)]
	r#type: ButtonType,
	/// Additional class names to apply to the link, if any
	#[props(into, optional)]
	class: String,
	/// Color of the link
	#[props(into, optional)]
	color: Color,
	/// Whether the button is disabled or not
	#[props(into, optional)]
	disabled: bool,
	/// Variant of the Link
	#[props(into, optional)]
	variant: LinkStyleVariant,
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: Element,
) -> Element {
	let class = format!(
		"flex items-center justify-center {} {}",
		class,
		match variant {
			LinkStyleVariant::Outlined => "btn-outline".to_string(),
			LinkStyleVariant::Contained => format!("btn btn-{}", color),
			_ => format!("btn-plain text-{}", color).to_string(),
		},
	);

	rsx! {
		button { r#type: r#type.to_string(), disabled, class, {children} }
	}
}
