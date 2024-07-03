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
	/// The Minimum value of the slider
	#[prop(into)]
	min: RwSignal<u16>,
	/// The Maximum value of the slider
	#[prop(into)]
	max: RwSignal<u16>,
	/// Minimum Possible Value of the input
	min_limit: u16,
	/// Maximum Possible Value of the input
	max_limit: u16,
) -> impl IntoView {
	let outer_div_class =
		class.with(|cname| format!("slider full-width fr-ct-ct pos-rel pb-xl {}", cname));

	let get_percent = move |val: u16| ((val - min_limit) / (max_limit - min_limit)) * 100;
	create_effect(move |_| {
		logging::log!("{} {}", get_percent(min.get()), get_percent(max.get()));
	});

	view! {
		<div class={outer_div_class}>
			<input
				type="range"
				prop:value={min}
				min={min_limit}
				min={max_limit}
				class="thumb full-width pos-abs left"
			/>
			<input
				type="range"
				prop:value={max}
				min={min_limit}
				min={max_limit}
				class="thumb full-width pos-abs right"
			/>

			<div class="pos-rel full-width txt-white">
				<div class="track pos-abs bg-secondary full-width br-sm"></div>
				{
					move || view! {
						<div
							style={format!(
								"right: {}%; left: {}%;",
								(100 - get_percent(max.get())),
								get_percent(min.get())
							)}
							class="range pos-abs br-sm bg-primary"
						></div>
					}
				}
			</div>
		</div>
	}
}
