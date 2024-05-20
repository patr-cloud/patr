use crate::imports::*;

/// A Slider for selecting both minimum and maximum values
/// uses two input[type=range] to facilitate the slider,
/// and show bobs and the ends
/// and an empty div[class="range"] to visually show the range
/// the left and the right css values of the same are modified to
/// stretch or compress it in either direction
#[component]
pub fn DoubleInputSlider(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Minimum value of the silder
	#[prop(default = 1)]
	min: usize,
	/// The Maximum value of the silder
	#[prop(default = 10)]
	max: usize,
	/// The Default MIN value of the slider
	#[prop(default = 1)]
	default_min: usize,
	/// The Default MAX value of the slider
	#[prop(default = 10)]
	default_max: usize,
) -> impl IntoView {
	let outer_div_class =
		class.with(|cname| format!("slider full-width fr-ct-ct pos-rel pb-xl {}", cname,));

	// The current min value of the slider
	let min_value = create_rw_signal(default_min);
	// The current max value of the slider
	let max_value = create_rw_signal(default_max);

	view! {
		<div class=outer_div_class>
			<input
				r#type="range"
				prop:value=min_value
				min=min
				min=max
				class="thumb full-width pos-abs left"
			/>
			<input
				r#type="range"
				prop:value=max_value
				min=min
				min=max
				class="thumb full-width pos-abs right"
			/>

			<div class="pos-rel full-width txt-white">
				<div class="track pos-abs bg-secondary full-width br-sm" />
				<div
					style="right: 0%; left: 0%;"
					class="range pos-abs br-sm bg-primary"
				/>
			</div>
		</div>
	}
}
