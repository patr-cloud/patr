use crate::prelude::*;

#[component]
pub fn ProfileCard(
	/// The click handler for the profile card
	on_click: Box<dyn Fn(&ev::MouseEvent)>,
	/// Additional class names to apply to the profile card, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let first_name = "Rakshith"; // TODO get this from user data
	let last_name = "Ravi"; // TODO get this from user data

	view! {
		<Link
			on_click={on_click}
			class={MaybeSignal::derive(move || {
				format!("fr-fe-ct px-lg row-card br-lg bg-secondary-light ml-sm {}", class.get())
			})}
		>

			<strong class="txt-of-ellipsis txt-medium txt-sm mr-md of-hidden w-30">
				{first_name} {last_name}
			</strong>
			<Avatar size={Size::Small} first_name={first_name} last_name={last_name}/>
		</Link>
	}
}
