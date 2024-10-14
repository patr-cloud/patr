use crate::prelude::*;

#[component]
pub fn ImageTag(
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Tag content
	tag: String,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| {
			format!(
				"px-xs py-xxs flex justify-start items-center bg-secondary-medium br-sm text-white {}",
				cname
			)
		})
	};
	view! {
		<div class={class}>
			<Icon icon={IconType::Tag} size={Size::ExtraSmall} class="mr-sm" />
			{tag}

			<button class="btn-icon ml-xs">
				<Icon icon={IconType::Copy} size={Size::ExtraSmall} />
			</button>
		</div>
	}
}
