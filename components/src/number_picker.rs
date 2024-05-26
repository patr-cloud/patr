use crate::imports::*;

#[component]
pub fn NumberPicker(
	/// Additional classes to apply to the outer div if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Secondary Color Variant
	#[prop(optional, default = SecondaryColorVariant::Light)]
	style_variant: SecondaryColorVariant,
	/// The min value of the input
	#[prop(optional, default = 0)]
	min: usize,
	/// The max value of the input
	#[prop(optional, default = 0)]
	max: usize,
	/// The Initial Value
	value: usize,
) -> impl IntoView {
	let value = create_rw_signal(value);

	let outer_div_class = move || {
		class.with(|cname| {
			format!(
				"number-picker row-card fr-sb-ct br-sm px-sm bg-secondary-{} {}",
				style_variant.as_css_name(),
				cname,
			)
		})
	};
	view! {
		<div class={outer_div_class}>
			<button class="btn-icon" r#type="button" aria_label="Minus Button">
				<Icon icon={IconType::Minus}/>
			</button>

			<input
				class="mx-md txt-white txt-center outline-primary-focus py-xxs br-sm"
				r#type="number"
				min={min}
				max={max}
				prop:value={value}
			/>

			<button class="btn-icon" r#type="button" aria_label="Plus Button">
				<Icon icon={IconType::Plus}/>
			</button>
		</div>
	}
}
