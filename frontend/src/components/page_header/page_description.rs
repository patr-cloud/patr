use crate::prelude::*;

/// Provides a brief description of the page, and optionally the doc link
#[component]
pub fn PageDescription(
	/// Additional Class Names to apply to the \<p/> tag, if any
	#[prop(into, optional)]
	class: Signal<String>,
	/// Description of the page content
	#[prop(into)]
	description: String,
	/// Link to the documentation
	#[prop(into, optional)]
	doc_link: Signal<Option<String>>,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex justify-start items-baseline fr-fs-bl text-grey mx-md text-sm {}",
			class.get()
		)
	};

	let doc_link = move || {
		doc_link.get().map(|link| {
			view! {
				<a
					class="btn-plain text-sm flex justify-start items-center"
					target="_blank"
					rel="noreferrer"
					href={link}
				>
					"Documentation"

					<Icon
						icon={IconType::ExternalLink}
						size={Size::ExtraExtraSmall}
						color={Color::Primary}
					/>
				</a>
			}
			.into_view()
		})
	};

	view! { <p class={class}>{description} {doc_link}</p> }
}
