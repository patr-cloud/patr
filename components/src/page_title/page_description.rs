use crate::imports::*;

/// Provides a breif description of the page, and optionally the doc link
#[component]
pub fn PageDescription(
	/// Additional classnames to appy to the \<p/> tag, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Description of the page content
	#[prop(into)]
	description: String,
	/// Link to the documentation
	#[prop(into, optional)]
	doc_link: MaybeSignal<Option<String>>,
) -> impl IntoView {
	let class = move || format!("flex justify-start fr-fs-bl txt-grey mx-md {}", class.get());

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
