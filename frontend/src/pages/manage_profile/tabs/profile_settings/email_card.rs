use crate::prelude::*;

#[component]
pub fn EmailCard(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The email to display
	#[prop(into)]
	email: MaybeSignal<String>,
) -> impl IntoView {
	let outer_div_class =
		class.with(|cname| format!("w-full flex items-center justify-start {}", cname));

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-11">
				<Textbox color_variant={SecondaryColorVariant::Medium} value={email.into_view()} />
			</div>

			<div class="flex-col-1 flex items-center justify-center">
				<button class="btn-icon" aria_label="Delete Email">
					<Icon icon={IconType::Trash2} color={Color::Error} />
				</button>
			</div>
		</div>
	}
}
