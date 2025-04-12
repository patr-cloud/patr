use crate::prelude::*;

#[component]
pub fn Button(
	/// Button Variant i.e. a button or a Link,
	/// Defaults to Button
	#[prop(into, optional)]
	r#type: Signal<ButtonType>,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: Signal<String>,
	/// Color of the link
	#[prop(into, optional)]
	color: Signal<Color>,
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: ChildrenFn,
	/// Whether the button is disabled or not
	#[prop(into, optional)]
	disabled: Signal<bool>,
	/// Variant of the Link
	#[prop(into, optional)]
	variant: Signal<LinkStyleVariant>,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex items-center justify-center {} {}",
			class.get(),
			match variant.get() {
				LinkStyleVariant::Outlined => "btn-outline".to_string(),
				LinkStyleVariant::Contained => format!("btn btn-{}", color.get()),
				_ => format!("btn-plain text-{}", color.get()).to_string(),
			},
		)
	};

	view! {
		<button
			type={r#type.with(|val| val.to_string())}
			disabled={disabled}
			class={class}
		>
			{children()}
		</button>
	}
}
