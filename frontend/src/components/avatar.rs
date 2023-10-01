use crate::prelude::*;

/// The avatar component, used for displaying a user's avatar.
#[component]
pub fn Avatar(
	/// The first name of the user.
	#[prop(into, optional)]
	first_name: MaybeSignal<String>,
	/// The last name of the user.
	#[prop(into, optional)]
	last_name: MaybeSignal<String>,
	/// The image of the user, if any
	#[prop(into, optional)]
	image: MaybeSignal<String>,
	/// The size of the avatar.
	#[prop(into, optional, default = Small.into())]
	size: MaybeSignal<Size>,
	/// Additional classes to add to the avatar, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The variant of the avatar, if any.
	#[prop(into, optional)]
	variant: MaybeSignal<Option<SecondaryColorVariant>>,
) -> impl IntoView {
	view! {
		<div class=move || format!(
			concat!(
				"avatar cursor-pointer txt-white bg-secondary",
				" fr-ct-ct of-hidden avatar-{} {} {}"
			),
			size.get().as_css_name(),
			if let Some(variant) = variant.get() {
				format!("bg-secondary-{}", variant.as_css_name())
			} else {
				"bg-secondary".to_owned()
			},
			class.get()
		)>
			{image
				.get()
				.some_if_not_empty()
				.map(|image| {
					let first_name = first_name.clone();
					view! {
						<img
							class="img-res"
							src={image}
							alt={
								first_name
								.get()
								.some_if_not_empty()
								.unwrap_or("avatar".into())
							}
						/>
					}
				})}
			{move || first_name
				.get()
				.some_if_not_empty()
				.map(|first_name| {
					view! {
						{first_name
							.chars()
							.next()
							.unwrap_or_default()
							.to_ascii_uppercase()}
					}
				})}
			{move || last_name
				.get()
				.some_if_not_empty()
				.map(|last_name| {
					view! {
						{last_name
							.chars()
							.next()
							.unwrap_or_default()
							.to_ascii_uppercase()}
					}
				})}
		</div>
	}
}
