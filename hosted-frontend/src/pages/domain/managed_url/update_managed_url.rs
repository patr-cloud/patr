use std::rc::Rc;

use crate::prelude::*;

#[component]
pub fn UpdateManagedUrl(
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Signal that sets whether to show the update card or not
	show_update_component: RwSignal<bool>,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| {
			format!(
				"full-width fc-fs-fs bg-secondary-light txt-white px-xl py-md {}",
				cname
			)
		})
	};

	view! {
		<form class=class>
			<div class="flex py-xxs full-width fr-fs-fs">
				<div class="flex-col-3 br-sm py-sm px-xl bg-secondary-medium">
					<div class="px-sm txt-disabled">"@"</div>
				</div>

				<div class="flex-col-6 fr-fs-fs pr-lg">
					<span class="mx-md txt-xl">"."</span>
					<div class="br-sm py-sm px-xl bg-secondary-medium full-width">
						<div class="px-sm txt-disabled">
							"betterheroku.com"
						</div>
					</div>
				</div>

				<div class="flex-col-3 fc-fs-fs">
					<Input
						class="full-width"
						placeholder="Add Path"
						r#type=InputType::Text
						variant=SecondaryColorVariant::Medium
					/>
				</div>
			</div>

			<div class="fr-fs-ct mt-lg ml-auto">
				<Link on_click=Rc::new(move |_| {
					show_update_component.set(false)
				}) class="btn mr-xs">
					"CANCEL"
				</Link>

				<Link
					style_variant=LinkStyleVariant::Contained
					should_submit=true
				>
					"UPDATE"
				</Link>
			</div>
		</form>
	}
}
