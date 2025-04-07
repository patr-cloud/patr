use std::cmp;

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
	#[prop(optional, default = 1)]
	min: u16,
	/// The max value of the input
	#[prop(optional, default = 10)]
	max: u16,
	/// The Initial Value
	#[prop(into)]
	value: RwSignal<u16>,
	/// Function to call on changing the value
	#[prop(into, optional, default = Callback::new(|_| {}))]
	on_change: Callback<()>,
) -> impl IntoView {
	let outer_div_class = move || {
		class.with(|cname| {
			format!(
				"number-picker row-card flex justify-between items-center br-sm px-sm bg-secondary-{} {}",
				style_variant.as_css_name(),
				cname,
			)
		})
	};

	let on_minus = move || {
		value.update(|v| {
			let changed_val = *v - 1;
			*v = cmp::max(cmp::min(changed_val, max), min)
		})
	};
	let on_plus = move || {
		value.update(|v| {
			let changed_val = *v + 1;

			*v = cmp::max(cmp::min(changed_val, max), min)
		})
	};

	view! {
		<div class={outer_div_class}>
			<button
				class="btn-icon"
				type="button"
				aria_label="Minus Button"
				on:click={move |_| {
					on_minus();
					on_change.call(());
				}}
			>
				<Icon icon={IconType::Minus}/>
			</button>

			<input
				class="mx-md text-white text-center outline-primary-focus py-xxs br-sm"
				type="number"
				min={min}
				max={max}
				prop:value={value}
			/>

			<button
				class="btn-icon"
				type="button"
				aria_label="Plus Button"
				on:click={move |_| {
					on_plus();
					on_change.call(());
				}}
			>
				<Icon icon={IconType::Plus}/>
			</button>
		</div>
	}
}
