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
	let outer_div_class = class.with(|cname| {
		format!(
			"slider w-full flex justify-center items-center relative pb-xl {}",
			cname
		)
	});

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
				class="thumb w-full absolute left"
			/>
			<input
				type="range"
				prop:value={max}
				min={min_limit}
				min={max_limit}
				class="thumb w-full absolute right"
			/>

			<div class="relative w-full text-white">
				<div class="track absolute bg-secondary w-full br-sm"></div>
				{
					move || view! {
						<div
							style={format!(
								"right: {}%; left: {}%;",
								(100 - get_percent(max.get())),
								get_percent(min.get())
							)}
							class="range absolute br-sm bg-primary"
						></div>
					}
				}
			</div>
		</div>
	}
}
