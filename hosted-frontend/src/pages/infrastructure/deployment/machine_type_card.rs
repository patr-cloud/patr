use crate::prelude::*;

#[component]
pub fn MachineTypeCard(
	/// Additional classes to apply to the outer div if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let outer_div_class = move || {
		class.with(|cname| {
			format!(
                "px-xl py-lg bg-secondary-medium cursor-pointer br-sm fc-fs-fs machine-type-card {}",
				cname,
			)
		})
	};
	view! {
		<div class=outer_div_class>
			<div class="fc-fs-fs full-width mb-xxs">
				<div class="fr-ct-ct">
					<span class="txt-primary txt-lg txt-regular">
						"$5"
					</span>

					<span class="txt-xs letter-sp-md">"/mo"</span>
				</div>
			</div>
			<div class="fr-fs-bl">
				<span class="txt-lg">"1 GB"</span>
				<span class="txt-disabled ml-xxs txt-xxs">RAM</span>
			</div>
			<div class="fr-fs-bl">
				<span class="txt-lg">"4"</span>
				<span class="txt-disabled ml-xxs txt-xxs">vCPU</span>
			</div>

		</div>
	}
}
